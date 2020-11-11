use bevy::prelude::*;
use bevy_egui::prelude::*;

fn main() {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::WARN)
            .finish(),
    )
    .expect("Failed to configure logging");

    App::build()
        // .add_resource(Msaa { samples: 4 })
        .add_default_plugins()
        .add_plugin(EguiPlugin)
        .add_startup_system(setup.system())
        .add_system(egui_test_system.system())
        .run();
}

fn egui_test_system(
    mut egui: ResMut<EguiUi>,
) {
    let ui = match egui.ui.as_mut() {
        None => return,
        Some(ui) => ui
    };

    let mut value = 1.0;

    egui::Window::new("Debug").fixed_pos([500.0, 200.0]).show(ui.ctx(), |ui| {
        ui.label(format!("Hello, world {}", 123));
        ui.add(egui::widgets::Label::new("Hello world!").heading());
        if ui.add(egui::widgets::Button::new("Save").fill(Some(egui::Rgba::RED.into()))).clicked {
            println!("Save clicked!");
        }
        ui.add(egui::Slider::f32(&mut value, 0.0..=1.0).text("float"));
    });
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
            mesh: meshes.add(Mesh::from(shape::Plane { size: 50.0 })),
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
            material: materials.add(Color::rgb(0.1, 0.4, 0.8).into()),
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
            transform: Transform::new(Mat4::face_toward(
                Vec3::new(-3.0, 5.0, 8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
