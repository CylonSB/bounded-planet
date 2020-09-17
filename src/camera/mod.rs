use bevy::prelude::*;
use smallvec::SmallVec;

/// A component to mark [`Camera3dComponents`] as cameras to be affected by
/// this plugin.
pub struct CameraBP;

/// The events/actions for a [`CameraBP`] to perform.
///
/// For variants with an `Option<f32>`, the field specifies the weight of the
/// of the action, or whether it's simply a signal.
///
/// As an example, if multiple `MoveLeft(None)`s are pushed to the event queue,
/// it's treated as if only a single `MoveLeft(None)` was pushed. On the other
/// hand, when multiple `MoveLeft(Some(_))` are pushed, their weights are
/// summed to get the final weight, `+ 1.0` if there was a `MoveLeft(None)`.
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CameraBPAction {
    /*PanLeft,
    PanRight,
    PanUp,
    PanDown,*/
    /// Translate the camera left.
    MoveLeft(Option<f32>),
    /// Translate the camera right.
    MoveRight(Option<f32>),
    /// Translate the camera in the direction it faces.
    MoveForward(Option<f32>),
    /// Translate the camera opposite the direction it faces.
    MoveBack(Option<f32>),
}

impl CameraBPAction {
    /// Return whether this is a signaling (ie `None`) action.
    pub const fn is_signal(&self) -> bool {
        match self {
            CameraBPAction::MoveLeft(None)
            | CameraBPAction::MoveRight(None)
            | CameraBPAction::MoveForward(None)
            | CameraBPAction::MoveBack(None) => true,
            _ => false
        }
    }

    /// Returns whether `self` and `other` are both signals for the same
    /// variant.
    pub fn both_same_signal(&self, other: &Self) -> bool {
        self.is_signal() && self == other 
    }

    /// Return a [`Vec<CameraBPAction>`] such that every signal in the
    /// collection is deduplicated.
    ///
    /// All signals are at the end of the returned `Vec`, in their order of
    /// initial appearance. All non-signals have their order retained.
    pub fn dedup_signals<I: IntoIterator<Item=Self>>(iter: I) -> Vec<Self> {
        // a little bit future-proofed
        const SIGNAL_TYPES: usize = 10;
        let mut signals = SmallVec::<[CameraBPAction; SIGNAL_TYPES]>::new();
        
        let (sigs, mut acts): (Vec<_>, Vec<_>) = iter.into_iter()
            .partition(| act | act.is_signal());
        
        for sig in sigs.into_iter() {
            if !signals.contains(&sig) {
                signals.push(sig)
            }
        }

        acts.extend_from_slice(signals.as_slice());
        acts
    }
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
    fn get_camspace_vec3_trans(&self, act: CameraBPAction) -> Option<Translation> {
        match act {
            CameraBPAction::MoveLeft(w) =>
                Some(Translation::new(w.unwrap_or(1.0) * self.left, 0.0, 0.0)),
            CameraBPAction::MoveRight(w) =>
                Some(Translation::new(w.unwrap_or(1.0) * self.right, 0.0, 0.0)),
            CameraBPAction::MoveForward(w) =>
                Some(Translation::new(0.0, 0.0, w.unwrap_or(1.0) * self.forward)),
            CameraBPAction::MoveBack(w) =>
                Some(Translation::new(0.0, 0.0, w.unwrap_or(1.0) * self.back)),
            _ => None
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
        app.add_resource::<InternalUG>(self.geo.into())
            .add_resource(self.cam_act)
            .add_resource::<CameraBPLocked>(self.locked.into())
            .add_event::<CameraBPAction>()
            .add_system(perform_camera_actions.system());
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

    let actions = CameraBPAction::dedup_signals(acts.get_reader().iter(&acts).copied());

    for (mut cam_t, mut cam_r) in cams.iter().into_iter() {
        for act in &actions {
            if let Some(t) = weights.get_camspace_vec3_trans(*act) {
                let (t, r) = res.0.trans(*cam_t, *cam_r, t);
                *cam_t = t;
                *cam_r = r;
            }
        }
    }
}
