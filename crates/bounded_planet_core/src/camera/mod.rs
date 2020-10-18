use bevy::prelude::*;

const DEFAULT_TRANS_SCALE: f32 = 0.2;
const DEFAULT_ZOOM_SCALE: f32 = 0.2;

// constants for setting default act amounts
const DEFAULT_MOVE_LEFT_AMOUNT: f32 = -0.1;
const DEFAULT_MOVE_RIGHT_AMOUNT: f32 = -DEFAULT_MOVE_LEFT_AMOUNT;
const DEFAULT_MOVE_FORWARD_AMOUNT: f32 = -0.1;
const DEFAULT_MOVE_BACK_AMOUNT: f32 = -DEFAULT_MOVE_FORWARD_AMOUNT;
const DEFAULT_ZOOM_IN_AMOUNT: f32 = -0.1;
const DEFAULT_ZOOM_OUT_AMOUNT: f32 = -DEFAULT_ZOOM_IN_AMOUNT;

type GTr = GlobalTransform;
type Tr = Transform;

/// A component to mark [`Camera3dComponents`] as cameras to be affected by
/// this plugin.
#[derive(Debug, Copy, Clone)]
pub struct CameraBPConfig {
    /// How much the distance between `geo` and the camera affects
    /// translational camera movements.
    pub trans_scale: f32,
    /// How much the distance between `geo` and the camera affects zooming
    /// camera movements.
    pub zoom_scale: f32,
    /// The scalar weight of [`CameraBPAction::MoveLeft`].
    pub left_weight: f32,
    /// The scalar weight of [`CameraBPAction::MoveRight`].
    pub right_weight: f32,
    /// The scalar weight of [`CameraBPAction::MoveForward`].
    pub forward_weight: f32,
    /// The scalar weight of [`CameraBPAction::MoveBack`].
    pub back_weight: f32,
    /// The scalar weight of [`CameraBPAction::ZoomIn`].
    pub zoomin_weight: f32,
    /// The scalar weight of [`CameraBPAction::Zoomout`].
    pub zoomout_weight: f32,
    /// Whether the camera is locked (unaffected by [`CameraBPAction`]s).
    pub locked: bool,
}

impl Default for CameraBPConfig {
    fn default() -> Self {
        Self {
            trans_scale: DEFAULT_TRANS_SCALE,
            zoom_scale: DEFAULT_ZOOM_SCALE,
            left_weight: DEFAULT_MOVE_LEFT_AMOUNT,
            right_weight: DEFAULT_MOVE_RIGHT_AMOUNT,
            forward_weight: DEFAULT_MOVE_FORWARD_AMOUNT,
            back_weight: DEFAULT_MOVE_BACK_AMOUNT,
            zoomin_weight: DEFAULT_ZOOM_IN_AMOUNT,
            zoomout_weight: DEFAULT_ZOOM_OUT_AMOUNT,
            locked: false,
        }
    }
}

impl CameraBPConfig {
    fn get_camspace_vec3_trans(&self, act: CameraBPAction) -> Option<Vec3> {
        match act {
            CameraBPAction::MoveLeft(w) => Some(Vec3::new(
                w.unwrap_or(1.0) * self.left_weight,
                0.0,
                0.0,
            )),
            CameraBPAction::MoveRight(w) => Some(Vec3::new(
                w.unwrap_or(1.0) * self.right_weight,
                0.0,
                0.0,
            )),
            CameraBPAction::MoveForward(w) => Some(Vec3::new(
                0.0,
                0.0,
                w.unwrap_or(1.0) * self.forward_weight,
            )),
            CameraBPAction::MoveBack(w) => Some(Vec3::new(
                0.0,
                0.0,
                w.unwrap_or(1.0) * self.back_weight,
            )),
            _ => None,
        }
    }

    fn get_camspace_vec3_zoom(&self, act: CameraBPAction) -> Option<f32> {
        match act {
            CameraBPAction::ZoomIn(w) => Some(w.unwrap_or(1.0) * self.zoomin_weight),
            CameraBPAction::ZoomOut(w) => Some(w.unwrap_or(1.0) * self.zoomout_weight),
            _ => None,
        }
    }
}

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
    /// Zoom in the camera.
    ZoomIn(Option<f32>),
    /// Zoom out the camera.
    ZoomOut(Option<f32>),
}

impl CameraBPAction {
    /// Return whether this is a signaling (ie `None`) action.
    pub const fn is_signal(&self) -> bool {
        matches!(self,
            CameraBPAction::MoveLeft(None)
            | CameraBPAction::MoveRight(None)
            | CameraBPAction::MoveForward(None)
            | CameraBPAction::MoveBack(None)
            | CameraBPAction::ZoomIn(None)
            | CameraBPAction::ZoomOut(None)
        )
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
    pub fn dedup_signals<I: IntoIterator<Item = Self>>(iter: I) -> Vec<Self> {
        // a little bit future-proofed
        const SIGNAL_TYPES: usize = 10;
        let mut signals = Vec::with_capacity(SIGNAL_TYPES);

        let (sigs, mut acts): (Vec<_>, Vec<_>) = iter.into_iter().partition(|act| act.is_signal());
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
/// Do **not** put this as a child to anything.
///
/// Cameras with a [`CameraBPConfig`], when all of their ancestor entities with
/// [`Transform`] components are taken into account, are assumed to be
/// transformed relative to one of the variants of this. For example, if the
/// universal geometry is a [`UniversalGeometry::Plane`], then cameras will be
/// translated, rotated, etc. wrt. the plane that's defined.
#[derive(Debug, Copy, Clone)]
pub enum UniversalGeometry {
    Plane { origin: Vec3, normal: Vec3 },
}

impl Default for UniversalGeometry {
    fn default() -> Self {
        UniversalGeometry::Plane {
            origin: Vec3::new(0.0, 0.0, 0.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
        }
    }
}

impl UniversalGeometry {
    /// Normalize the given [`UniversalGeometry`] so that it satisfies the
    /// invariants required by the internal camera state.
    #[allow(irrefutable_let_patterns)] // only because UG is Plane
    pub fn normalize(self) -> Self {
        if let UniversalGeometry::Plane { normal, .. } = &self {
            debug_assert!(
                normal.length().abs() > std::f32::EPSILON,
                "Got normal with zero length"
            );
        }

        match self {
            UniversalGeometry::Plane { origin, normal } => UniversalGeometry::Plane {
                origin,
                normal: normal.normalize(),
            },
        }
    }
}

fn compose(l: &Tr, r: &Tr) -> Tr {
    Tr::new(*l.value() * *r.value())
}

fn inverse(l: &Tr) -> Tr {
    Tr::new(l.value().inverse())
}

fn composeg(l: &Tr, r: &GTr) -> GTr {
    GTr::new(*l.value() * *r.value())
}

/// A private newtype of Universal Geometry, that satisfies some invariants:
/// 1) `self.0` is normalized.
/// 2) If `self.0` is a plane, then its normal is a unit vector.
#[derive(Debug, Copy, Clone)]
struct InternalUG(UniversalGeometry);

impl From<UniversalGeometry> for InternalUG {
    fn from(ug: UniversalGeometry) -> Self {
        InternalUG(ug)
    }
}

impl InternalUG {
    fn trans_plane(n: Vec3, ogt: GTr, s: Vec3, scale: f32) -> Tr {
        let mut delta = ogt.rotation().mul_vec3(s);
        delta -= n * delta.dot(n);

        // when delta is zero, delta.normalize() is (NaN, NaN, NaN), which causes camera to die
        if delta != Vec3::new(0.0, 0.0, 0.0) {
            delta = delta.normalize() * s.length(); // unscaled delta
            delta *= scale * ogt.translation().dot(n).abs().max(0.001); // scale delta by dist
        }

        Tr::from_translation(delta)
    }

    /// Get the new [`Transform`] and [`GlobalTransform`] resulting from the
    /// original [`GlobalTransform`] `ot` and movement `s` in terms of
    /// [`InternalUG`] space.
    fn noparent_trans(&self, ot: &mut Tr, ogt: &mut GTr, s: Vec3, scale: f32) {
        match self.0 {
            UniversalGeometry::Plane { normal, .. } => {
                let dt = Self::trans_plane(normal, *ogt, s, scale); // delta transform
                *ot = compose(&dt, ot);
                *ogt = composeg(&dt, ogt);
            }
        }
    }

    fn zoom_plane(n: Vec3, ogt: GTr, s: f32, scale: f32) -> Tr {
        let mut delta = -ogt.rotation() * Vec3::new(0.0, 0.0, s); // unscaled delta
        delta *= scale * ogt.translation().dot(n).abs().max(0.001); // scale delta by dist

        Tr::from_translation(delta)
    }

    /// Get the new transformation resulting from the original [`Transform`]
    /// `ot` and scroll weight `s`.
    fn noparent_zoom(&self, ot: &mut Tr, ogt: &mut GTr, s: f32, scale: f32) {
        match self.0 {
            UniversalGeometry::Plane { normal, ..} => {
                let dt = Self::zoom_plane(normal, *ogt, s, scale); // delta transform
                *ot = compose(&dt, ot);
                *ogt = composeg(&dt, ogt);
            }
        }
    }

    /// Return the `(parent, camera)` new transformations resulting from the
    /// original [`Transform`]s `opt` and `oct` of the parent and camera
    /// respectively and movement `s` in terms of [`InternalUG`] space.
    fn parent_trans(
        &self,
        opt: &mut Tr,
        opgt: &mut GTr,
        oct: &mut Tr,
        ocgt: &mut GTr,
        s: Vec3,
        scale: f32
    ) {
        match self.0 {
            UniversalGeometry::Plane { normal , ..} => {
                // oct * delta parent transform
                let cdt = Self::trans_plane(normal, *ocgt, s, scale);
                // delta parent transform
                let pdt = compose(&inverse(oct), &cdt);
                *opt = compose(&pdt, &opt);
                *opgt = composeg(&pdt, &opgt);
            }
        }
    }

    /// Return the `(parent, camera)` new transformations resulting from the
    /// original [`Transform`]s `opt` and `oct` of the parent and camera
    /// respectively and and scroll weight `s`.
    fn parent_zoom(
        &self,
        _opt: &mut Tr,
        _opgt: &mut GTr,
        oct: &mut Tr,
        ocgt: &mut GTr,
        s: f32,
        scale: f32
    ) {
        match self.0 {
            UniversalGeometry::Plane {..} => {
                self.noparent_zoom(oct, ocgt, s, scale);
            }
        }
    }
}

/// A plugin that adds an [`InternalUG`] and adds systems to control cameras
/// relative to it.
#[derive(Default)]
pub struct CameraBPPlugin {
    /// The geometry that the camera follows.
    pub geo: UniversalGeometry,
}

impl Plugin for CameraBPPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource::<InternalUG>(self.geo.into())
            .add_event::<CameraBPAction>()
            .add_system(perform_parentless_camera_actions.system())
            .add_system(perform_parented_camera_actions.system());
    }
}

/// Performs the camera actions pushed to the queue for cameras without
/// parents.
fn perform_parentless_camera_actions(
    acts: Res<Events<CameraBPAction>>,
    res: Res<InternalUG>,
    mut cams: Query<Without<Parent, (&CameraBPConfig, &mut Tr, &GTr)>>,
) {
    let actions = CameraBPAction::dedup_signals(acts.get_reader().iter(&acts).copied());

    for (bp, mut cam_t, cam_gt) in cams.iter().into_iter() {
        if bp.locked {
            continue;
        }

        let mut cam_gt = *cam_gt;

        for act in &actions {
            if let Some(t) = bp.get_camspace_vec3_trans(*act) {
                res.noparent_trans(&mut cam_t, &mut cam_gt, t, bp.trans_scale)
            } else if let Some(w) = bp.get_camspace_vec3_zoom(*act) {
                res.noparent_zoom(&mut cam_t, &mut cam_gt, w, bp.trans_scale)
            } else {
                continue;
            }
        }
    }
}

/// Performs the camera actions pushed to the queue for cameras without
/// parents.
fn perform_parented_camera_actions(
    acts: Res<Events<CameraBPAction>>,
    res: Res<InternalUG>,
    trans: Query<(&mut Tr, &GTr)>,
    mut cams: Query<(Entity, &Parent, &CameraBPConfig)>,
) {
    let actions = CameraBPAction::dedup_signals(acts.get_reader().iter(&acts).copied());

    for (me, parent, bp) in cams.iter().into_iter() {
        let cam_t = trans.get_mut::<Tr>(me);
        let cam_gt = trans.get::<GTr>(me);
        let par_t = trans.get_mut::<Tr>(parent.0);
        let par_gt = trans.get::<GTr>(parent.0);

        let (mut cam_t, mut cam_gt) = match (cam_t, cam_gt) {
            _ if bp.locked => continue,
            (Ok(tf), Ok(gtf)) => (tf, *gtf),
            (tf, gtf) => {
                eprintln!("Couldn't get my own Transform or GlobalTransform!");
                eprintln!("My tf: {:?}\nMy gtf: {:?}", tf, gtf);
                panic!();
            }
        };

        let (mut par_t, mut par_gt) = match (par_t, par_gt) {
            (Ok(tf), Ok(gtf)) => (tf, *gtf),
            (tf, gtf) => {
                eprintln!("Couldn't get parent's own Transform or GlobalTransform!");
                eprintln!("Parent tf: {:?}\nParent gtf: {:?}", tf, gtf);
                panic!();
            }
        };

        for act in &actions {
            if let Some(t) = bp.get_camspace_vec3_trans(*act) {
                res.parent_trans(&mut par_t, &mut par_gt, &mut cam_t, &mut cam_gt, t, bp.trans_scale)
            } else if let Some(w) = bp.get_camspace_vec3_zoom(*act) {
                res.parent_zoom(&mut par_t, &mut par_gt, &mut cam_t, &mut cam_gt, w, bp.trans_scale)
            } else {
                continue;
            }
        }
    }
}
