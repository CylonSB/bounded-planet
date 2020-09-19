use bevy::{
    prelude::*,
    render::mesh::shape,
    input::mouse::{
        MouseWheel,
        MouseScrollUnit
    }
};
use bounded_planet::{
    camera::*,
    land::*,
};

// The thresholds for window edge.
const CURSOR_H_THRESHOLD: f32 = 0.55;
const CURSOR_V_THRESHOLD: f32 = 0.42;

/// The stage at which the [`CameraBP`] cache is either updated or used to fill
/// in the action cache now.
const CAM_CACHE_UPDATE: &'static str = "push_cam_update";

#[derive(Default)]
struct MoveCam {
    right: Option<f32>,
    forward: Option<f32>
}

fn main() {
    App::build()
        .init_resource::<MoveCam>()
        .add_resource(Msaa { samples: 4 })
        .add_default_plugins()
        .add_plugin(CameraBPPlugin {
            geo: UniversalGeometry::Plane {
                origin: Translation::identity(),
                normal: Vec3::new(0.0, 1.0, 0.0)
            },
            ..Default::default()
        })
        .add_startup_system(setup.system())
        .add_system_to_stage(stage::EVENT_UPDATE, act_camera_on_window_edge.system())
        .add_system_to_stage(stage::EVENT_UPDATE, act_on_scroll_wheel.system())
        .add_stage_after(stage::EVENT_UPDATE, CAM_CACHE_UPDATE)
        .add_system_to_stage(CAM_CACHE_UPDATE, use_or_update_action_cache.system())
        .run();
}

/// set up a simple 3D scene with landscape?
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let land_texture_handle = asset_server
        .load_sync(&mut textures, "src/media/CoveWorld.png")
        .unwrap();

    let land_texture_top_handle = asset_server
        .load_sync(&mut textures, "src/media/CoveWorldTop.png")
        .unwrap();

    let land_mesh = texture_to_mesh(textures, land_texture_handle);

    commands.spawn(PbrComponents {
        mesh: meshes.add(land_mesh),
        material: materials.add(StandardMaterial {
            albedo_texture: Some(land_texture_top_handle),
            shaded: false,
            ..Default::default()
        }),
        translation: Translation::new(4.0, 1.5, 4.0),
        ..Default::default()
    });

    // add entities to the world
    commands
        // cube
        .spawn(PbrComponents {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.5, 0.4, 0.3).into()),
            translation: Translation::new(-20 as f32, 1.0, -20 as f32),
            ..Default::default()
        })
        // light
        .spawn(LightComponents {
            translation: Translation::new(4.0, 8.0, 4.0),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            translation: Translation::new(20.0, 20.0, 20.0),
            rotation: Rotation::from_rotation_xyz(-0.75, 2.7, 0.0),
            ..Default::default()
        }).with(CameraBPConfig {
            forward_weight: -0.01,
            back_weight: 0.01,
            left_weight: -0.01,
            right_weight: 0.01,
            ..Default::default()
        });
}

/// Pushes camera actions based upon mouse movements near the window edge.
fn act_camera_on_window_edge(
    wins: Res<Windows>,
    pos: Res<Events<CursorMoved>>,
    mut mcam: ResMut<MoveCam>
) {
    if let Some(e) = pos.get_reader().find_latest(&pos, | e | e.id.is_primary()) {
        let (mut mouse_x, mut mouse_y) = (e.position.x(), e.position.y());
        let window = wins.get(e.id).expect("Couldn't get primary window.");
        let (window_x, window_y) = (window.width as f32, window.height as f32);
        
        // map (mouse_x, mouse_y) into [-1, 1]^2
        mouse_x /= window_x / 2.0;
        mouse_y /= window_y / 2.0;
        mouse_x -= 1.0;
        mouse_y -= 1.0;
        let angle = mouse_x.atan2(mouse_y);
        let (ax, ay) = (angle.sin(), angle.cos());
        let in_rect = (-CURSOR_H_THRESHOLD <= mouse_x && mouse_x <= CURSOR_H_THRESHOLD)
            && (-CURSOR_V_THRESHOLD <= mouse_y && mouse_y <= CURSOR_V_THRESHOLD);
        
        if !in_rect && ax.is_finite() && ay.is_finite() {
            mcam.right = Some(ax);
            mcam.forward = Some(ay);
        } else {
            mcam.right = None;
            mcam.forward = None;
        }
    }
}

/// Pushes camera actions based upon scroll wheel movement.
fn act_on_scroll_wheel(
    mouse_wheel: Res<Events<MouseWheel>>,
    mut acts: ResMut<Events<CameraBPAction>>
) {
    for mw in mouse_wheel.get_reader().iter(&mouse_wheel) {
        /// If scrolling units are reported in lines rather than pixels,
        /// multiply the returned horizontal scrolling amount by this.
        const LINE_SIZE: f32 = 14.0;
        let w = mw.y.abs() * if let MouseScrollUnit::Line = mw.unit { LINE_SIZE } else { 1.0 };

        if mw.y > 0.0 {
            acts.send(CameraBPAction::ZoomIn(Some(w)))
        } else if mw.y < 0.0 {
            acts.send(CameraBPAction::ZoomOut(Some(w)))
        }
    }
}

/// Depending on `dirty`, either update the local `cache` or fill the event
/// queue for [`CameraBPAction`] with the locally cached copy.
fn use_or_update_action_cache(mcam: Res<MoveCam>, mut acts: ResMut<Events<CameraBPAction>>) {
    if let Some(w) = mcam.right {
        acts.send(CameraBPAction::MoveRight(Some(w)))
    }

    if let Some(w) = mcam.forward {
        acts.send(CameraBPAction::MoveForward(Some(w)))
    }
}
