use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};
use bounded_planet::camera::*;

/// The threshold for horizontal cursor-activated [`CameraBPConfig`] movement.
///
/// Is a proportion of the window size. So, if this is `0.05`, then the cursor
/// must be within 5% of the window size to either the left or right edge to
/// trigger this threshold.
const CURSOR_EDGE_H_THRESHOLD: f32 = 0.05;
/// The threshold for vertical cursor-activated [`CameraBPConfig`] movement.
///
/// Is a proportion of the window size. So, if this is `0.05`, then the cursor
/// must be within 5% of the window size to either the top or bottom edge to
/// trigger this threshold.
const CURSOR_EDGE_V_THRESHOLD: f32 = 0.05;

/// The stage at which the [`CameraBPConfig`] cache is either updated or used to fill
/// in the action cache now.
const CAM_CACHE_UPDATE: &str = "push_cam_update";

#[derive(Copy, Clone)]
struct IsActionCacheDirty(bool);

impl Default for IsActionCacheDirty {
    fn default() -> Self {
        IsActionCacheDirty(true)
    }
}

fn main() {
    App::build()
        .init_resource::<IsActionCacheDirty>()
        .add_resource(Msaa { samples: 4 })
        .add_default_plugins()
        .add_plugin(CameraBPPlugin::default())
        .add_startup_system(setup.system())
        .add_system_to_stage(stage::EVENT_UPDATE, act_camera_on_window_edge.system())
        .add_system_to_stage(stage::EVENT_UPDATE, act_on_scroll_wheel.system())
        .add_stage_after(stage::EVENT_UPDATE, CAM_CACHE_UPDATE)
        .add_system_to_stage(CAM_CACHE_UPDATE, use_or_update_action_cache.system())
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // add entities to the world
    commands
        // plane
        .spawn(PbrComponents {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 10.0 })),
            material: materials.add(Color::rgb(0.1, 0.2, 0.1).into()),
            ..Default::default()
        })
        // cube
        .spawn(PbrComponents {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.5, 0.4, 0.3).into()),
            transform: Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            ..Default::default()
        })
        // sphere
        .spawn(PbrComponents {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                subdivisions: 4,
                radius: 0.5,
            })),
            transform: Transform::from_translation(Vec3::new(1.5, 1.5, 1.5)),
            ..Default::default()
        })
        // light
        .spawn(LightComponents {
            transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::from_translation_rotation(
                Vec3::new(0.0, 7.0, 0.0),
                Quat::from_rotation_ypr(0.3, -0.8, -0.2)
            ),
            ..Default::default()
        })
        .with(CameraBPConfig::default());
}

/// Pushes camera actions based upon mouse movements near the window edge.
fn act_camera_on_window_edge(
    wins: Res<Windows>,
    mut dirty: ResMut<IsActionCacheDirty>,
    pos: Res<Events<CursorMoved>>,
    mut acts: ResMut<Events<CameraBPAction>>,
) {
    dirty.0 = false;

    if let Some(e) = pos.get_reader().find_latest(&pos, |e| e.id.is_primary()) {
        let (mouse_x, mouse_y) = (e.position.x(), e.position.y());
        let window = wins.get(e.id).expect("Couldn't get primary window.");
        let (window_x, window_y) = (window.width as f32, window.height as f32);
        dirty.0 = true;

        if mouse_x / window_x <= CURSOR_EDGE_H_THRESHOLD {
            acts.send(CameraBPAction::MoveLeft(None));
        }

        if 1.0 - mouse_x / window_x <= CURSOR_EDGE_H_THRESHOLD {
            acts.send(CameraBPAction::MoveRight(None));
        }

        if mouse_y / window_y <= CURSOR_EDGE_V_THRESHOLD {
            acts.send(CameraBPAction::MoveBack(None));
        }

        if 1.0 - mouse_y / window_y <= CURSOR_EDGE_V_THRESHOLD {
            acts.send(CameraBPAction::MoveForward(None));
        }
    }
}

/// Pushes camera actions based upon scroll wheel movement.
fn act_on_scroll_wheel(
    mouse_wheel: Res<Events<MouseWheel>>,
    mut acts: ResMut<Events<CameraBPAction>>,
) {
    for mw in mouse_wheel.get_reader().iter(&mouse_wheel) {
        /// If scrolling units are reported in lines rather than pixels,
        /// multiply the returned horizontal scrolling amount by this.
        const LINE_SIZE: f32 = 14.0;
        let w = mw.y.abs()
            * if let MouseScrollUnit::Line = mw.unit {
                LINE_SIZE
            } else {
                1.0
            };

        if mw.y > 0.0 {
            acts.send(CameraBPAction::ZoomIn(Some(w)))
        } else if mw.y < 0.0 {
            acts.send(CameraBPAction::ZoomOut(Some(w)))
        }
    }
}

// Return whether this action was created from the window edge.
fn is_winedge_act(act: &CameraBPAction) -> bool {
    match act {
        CameraBPAction::MoveLeft(_)
        | CameraBPAction::MoveRight(_)
        | CameraBPAction::MoveForward(_)
        | CameraBPAction::MoveBack(_) => true,
        _ => false,
    }
}

/// Depending on `dirty`, either update the local `cache` or fill the event
/// queue for [`CameraBPAction`] with the locally cached copy.
fn use_or_update_action_cache(
    mut cache: Local<Vec<CameraBPAction>>,
    mut acts: ResMut<Events<CameraBPAction>>,
    dirty: Res<IsActionCacheDirty>,
) {
    if dirty.0 {
        *cache = CameraBPAction::dedup_signals(
            acts.get_reader()
                .iter(&acts)
                .copied()
                .filter(is_winedge_act),
        );
    } else {
        acts.extend(cache.iter().copied())
    }
}
