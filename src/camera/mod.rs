use std::collections::HashSet;
use bevy::prelude::*;

/// The threshold for horizontal cursor-activated [`CameraBP`] movement.
///
/// Is a proportion of the window size. So, if this is `0.05`, then the cursor
/// must be within 5% of the window size to either the left or right edge to
/// trigger this threshold.
const CURSOR_EDGE_H_THRESHOLD: f32 = 0.05;
/// The threshold for vertical cursor-activated [`CameraBP`] movement.
///
/// Is a proportion of the window size. So, if this is `0.05`, then the cursor
/// must be within 5% of the window size to either the top or bottom edge to
/// trigger this threshold.
const CURSOR_EDGE_V_THRESHOLD: f32 = 0.05;

/// A component to mark [`Camera3dComponents`] as cameras to be affected by
/// this plugin.
pub struct CameraBP;

/// The events/actions for a [`CameraBP`] to perform.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
enum CameraBPAction {
    /*PanLeft,
    PanRight,
    PanUp,
    PanDown,*/
    /// Translate the camera left.
    MoveLeft,
    /// Translate the camera right.
    MoveRight,
    /// Translate the camera in the direction it faces.
    MoveForward,
    /// Translate the camera opposite the direction it faces.
    MoveBack,
    //MoveUp,
    //MoveDown,
}

impl CameraBPAction {
    /// The number of variants of [`CameraBPAction`].
    const VARIANTS: usize = 4;
}

/// The universal geometry that the camera moves upon.
///
/// Origin fields are expected to be wrt. the origin used by [`CameraBP`]s.
#[derive(Copy, Clone)]
pub enum UniversalGeometry {
    Plane { origin: Translation, normal: Vec3 }
}

impl Default for UniversalGeometry {
    fn default() -> Self {
        UniversalGeometry::Plane {
            origin: Translation::identity(),
            normal: Vec3::new(0.0, 1.0, 0.0)
        }
    }
}

impl UniversalGeometry {
    /// Normalize the given [`UniversalGeometry`] so that it satisfies the
    /// invariants required by the internal camera state.
    #[allow(irrefutable_let_patterns)]  // only because UG is Plane
    pub fn normalize(self) -> Self {
        if let UniversalGeometry::Plane { normal, .. } = &self {
            debug_assert!(
                normal.length().abs() > std::f32::EPSILON,
                "Got normal with zero length"
            );
        }

        match self {
            UniversalGeometry::Plane { origin, normal } =>
                UniversalGeometry::Plane { origin, normal: normal.normalize() }
        }
    }

    /// Get the new position and rotation resulting from the original position
    /// `p`, original rotation `r`, and movement `s` about `self` (relative to
    /// `r`).
    fn trans(&self, p: Translation, r: Rotation, s: Translation) -> (Translation, Rotation) {
        fn plane(_o: Vec3, n: Vec3, mut p: Vec3, r: Quat, s: Vec3) -> (Vec3, Quat) {
            let mut s2 = r.mul_vec3(s);
            s2 -= n * s2.dot(n);
            p += s2.normalize() * s.length();
            (p, r)
        }

        let (p, r) = match self {
            UniversalGeometry::Plane { origin, normal } => plane(origin.0, *normal, p.0, r.0, s.0)
        };

        (Translation(p), Rotation(r))
    }
}

// constants for setting default act amounts
const DEFAULT_MOVE_LEFT_AMOUNT: f32 = -0.1;
const DEFAULT_MOVE_RIGHT_AMOUNT: f32 = -DEFAULT_MOVE_LEFT_AMOUNT;
const DEFAULT_MOVE_FORWARD_AMOUNT: f32 = -0.1;
const DEFAULT_MOVE_BACK_AMOUNT: f32 = -DEFAULT_MOVE_FORWARD_AMOUNT;

/// The associated scalar "strengths"/weights of camera actions.
#[derive(Copy, Clone)]
pub struct CameraBPActAmount {
    pub left: f32,
    pub right: f32,
    pub forward: f32,
    pub back: f32
}

impl Default for CameraBPActAmount {
    fn default() -> Self {
        CameraBPActAmount {
            left: DEFAULT_MOVE_LEFT_AMOUNT,
            right: DEFAULT_MOVE_RIGHT_AMOUNT,
            forward: DEFAULT_MOVE_FORWARD_AMOUNT,
            back: DEFAULT_MOVE_BACK_AMOUNT
        }
    }
}

impl CameraBPActAmount {
    fn get_camspace_vec3_trans(&self, act: CameraBPAction) -> Translation {
        match act {
            CameraBPAction::MoveLeft => Translation::new(self.left, 0.0, 0.0),
            CameraBPAction::MoveRight => Translation::new(self.right, 0.0, 0.0),
            CameraBPAction::MoveForward => Translation::new(0.0, 0.0, self.forward),
            CameraBPAction::MoveBack => Translation::new(0.0, 0.0, self.back)
        }
    }
}

/// Whether the camera is locked (ie, can't be moved by player actions).
#[derive(Copy, Clone)]
pub struct CameraBPLocked(pub bool);

impl Default for CameraBPLocked {
    fn default() -> Self {
        CameraBPLocked(false)
    }
}

impl From<bool> for CameraBPLocked {
    fn from(l: bool) -> Self {
        CameraBPLocked(l)
    }
}

impl From<CameraBPLocked> for bool {
    fn from(l: CameraBPLocked) -> Self {
        l.0
    }
}

/// A private newtype of Universal Geometry, that satisfies some invariants:
/// 1) `self.0` is normalized.
/// 2) If `self.0` is a plane, then its normal is a unit vector.
#[derive(Copy, Clone)]
struct InternalUG(UniversalGeometry);

impl From<UniversalGeometry> for InternalUG {
    fn from(ug: UniversalGeometry) -> Self {
        InternalUG(ug)
    }
}

/// The stage at which the [`CameraBP`] cache is either updated or used to fill
/// in the action cache now.
const CAM_CACHE_UPDATE: &'static str = "push_cam_update";

#[derive(Copy, Clone)]
struct IsActionCacheDirty(bool);

impl Default for IsActionCacheDirty {
    fn default() -> Self {
        IsActionCacheDirty(true)
    }
}

/// A plugin that adds a [`CameraBP`] and adds systems to control it.
#[derive(Default)]
pub struct CameraBPPlugin {
    /// The geometry that the camera follows.
    pub geo: UniversalGeometry,
    /// The "strength" (such as pan speed) of camera actions.
    pub cam_act: CameraBPActAmount,
    /// Whether the camera is locked (ie, can't be moved by player actions).
    pub locked: bool
}

impl Plugin for CameraBPPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<IsActionCacheDirty>()
            .add_resource::<InternalUG>(self.geo.into())
            .add_resource(self.cam_act)
            .add_resource::<CameraBPLocked>(self.locked.into())
            .add_event::<CameraBPAction>()
            .add_system_to_stage(stage::EVENT_UPDATE, act_camera_on_window_edge.system())
            .add_stage_after(stage::EVENT_UPDATE, CAM_CACHE_UPDATE)
            .add_system_to_stage(CAM_CACHE_UPDATE, use_or_update_action_cache.system())
            .add_system(perform_camera_actions.system());
    }
}

/// Pushes camera actions based upon mouse movements near the window edge.
fn act_camera_on_window_edge(
    wins: Res<Windows>,
    mut dirty: ResMut<IsActionCacheDirty>,
    pos: Res<Events<CursorMoved>>,
    mut cams: ResMut<Events<CameraBPAction>>
) {
    dirty.0 = false;

    if let Some(e) = pos.get_reader().find_latest(&pos, | e | e.id.is_primary()) {
        let (mouse_x, mouse_y) = (e.position.x(), e.position.y());
        let window = wins.get(e.id).expect("Couldn't get primary window.");
        let (window_x, window_y) = (window.width as f32, window.height as f32);
        dirty.0 = true;

        if mouse_x / window_x <= CURSOR_EDGE_H_THRESHOLD {
            cams.send(CameraBPAction::MoveLeft);
        }

        if 1.0 - mouse_x / window_x <= CURSOR_EDGE_H_THRESHOLD {
            cams.send(CameraBPAction::MoveRight);
        }

        if mouse_y / window_y <= CURSOR_EDGE_V_THRESHOLD {
            cams.send(CameraBPAction::MoveBack);
        }

        if 1.0 - mouse_y / window_y <= CURSOR_EDGE_V_THRESHOLD {
            cams.send(CameraBPAction::MoveForward);
        }
    }
}

/// Depending on `dirty`, either update the local `cache` or fill the event
/// queue for [`CameraBPAction`] with the locally cached copy.
fn use_or_update_action_cache(
    mut cache: Local<Vec<CameraBPAction>>,
    mut cams: ResMut<Events<CameraBPAction>>,
    dirty: Res<IsActionCacheDirty>
) {
    if dirty.0 {
        *cache = cams.get_reader().iter(&cams).copied().collect();
        cache.dedup();
    } else {
        cams.extend(cache.iter().copied())
    }
}

/// Performs the camera actions pushed to the queue, for every [`CameraBP`].
fn perform_camera_actions(
    acts: Res<Events<CameraBPAction>>,
    res: Res<InternalUG>,
    weights: Res<CameraBPActAmount>,
    locked: Res<CameraBPLocked>,
    mut cams: Query<With<CameraBP, (&mut Translation, &mut Rotation)>>
) {
    if locked.0 {
        return;
    }

    let mut actions = HashSet::with_capacity(CameraBPAction::VARIANTS);

    for act in acts.get_reader().iter(&acts) {
        actions.insert(*act);
    }

    for (mut cam_t, mut cam_r) in cams.iter().into_iter() {
        for act in &actions {
            let (t, r) = res.0.trans(*cam_t, *cam_r, weights.get_camspace_vec3_trans(*act));

            *cam_t = t;
            *cam_r = r;
        }
    }
}
