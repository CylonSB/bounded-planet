use bevy::prelude::*;
use bevy_rapier3d::{
    physics::RapierPhysicsPlugin,
    render::RapierRenderPlugin,
    rapier::{
        dynamics::RigidBodyBuilder,
        geometry::ColliderBuilder,
    }
};

use bounded_planet::unit_selection::{UnitSelectionPlugin, UnitSelectionHighlighterPlugin};

fn main() {
    App::build()
        .add_resource(Msaa { samples: 4 })
        .add_default_plugins()
        .add_plugin(RapierPhysicsPlugin)
        .add_plugin(RapierRenderPlugin)
        .add_plugin(UnitSelectionPlugin)
        .add_plugin(UnitSelectionHighlighterPlugin)
        .add_startup_system(setup.system())
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
        .with(RigidBodyBuilder::new_static())
        .with(ColliderBuilder::cuboid(10.0, 0.0001, 10.0))

        // cube
        .spawn(PbrComponents {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.5, 0.4, 0.3).into()),
            ..Default::default()
        })
        .with(RigidBodyBuilder::new_dynamic().translation(0.0, 2.0, 0.0))
        .with(ColliderBuilder::cuboid(1.0, 1.0, 1.0))

        // sphere
        .spawn(PbrComponents {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                subdivisions: 4,
                radius: 0.5,
            })),
            material: materials.add(Color::rgb(0.1, 0.4, 0.8).into()),
            ..Default::default()
        })
        .with(RigidBodyBuilder::new_dynamic().translation(1.5, 1.5, 1.5))
        .with(ColliderBuilder::ball(0.5))

        // light
        .spawn(LightComponents {
            transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::new(Mat4::face_toward(
                Vec3::new(2.0, 12.0, 10.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
