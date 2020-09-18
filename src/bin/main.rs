use bevy::{
    prelude::*,
    render::{
        mesh::shape,
        pipeline::{DynamicBinding, PipelineDescriptor, PipelineSpecialization, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
    },
};



fn main() {
    App::build()
        .add_resource(Msaa { samples: 4 })
        .add_default_plugins()
        .add_asset::<LandMaterial>()
        .add_startup_system(setup.system())
        .run();
}

#[derive(RenderResources, Default)]
struct LandMaterial {
    pub color: Color,
}

const VERTEXSHADER: &str = include_str!("../media/shaders/land.vert");
const FRAGSHADER: &str = include_str!("../media/shaders/land.frag");

/// set up a simple 3D scene with landscape?
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut shader_materials: ResMut<Assets<LandMaterial>>,
    mut render_graph: ResMut<RenderGraph>, 
) {
    let land_pipeline = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex,VERTEXSHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGSHADER))),
    }));

    render_graph.add_system_node("land_material", AssetRenderResourcesNode::<LandMaterial>::new(true),);

    render_graph
        .add_node_edge("land_material", base::node::MAIN_PASS)
        .unwrap();

    let _land_material = shader_materials.add(LandMaterial {
        color: Color::rgb(0.0, 0.8, 0.0),
    });


    for x in -15..15 {
        for y in -15..15 {
            let scale = 5.0;
            let land_mesh = PbrComponents{
                mesh: asset_server.load("src/media/land.gltf").unwrap(),
                render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                    land_pipeline,
                    PipelineSpecialization {
                        dynamic_bindings: vec![
                            // Transform
                            DynamicBinding {
                                bind_group: 1,
                                binding: 0,
                            },
                        ],
                        ..Default::default()           
                    },
                )]),
                translation: Translation::new(x as f32, 0.0, y as f32),
                scale: Scale(scale),
                material: materials.add(Color::rgb(0.5, 0.4, 0.3).into()),
                ..Default::default()
            };

            commands.spawn(land_mesh).with(_land_material);  
        }
    }

    // add entities to the world
    commands
        // sphere
        .spawn(PbrComponents {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                subdivisions: 4,
                radius: 2.0,
            })),
            material: materials.add(Color::rgb(0.1, 0.4, 0.8).into()),
            translation: Translation::new(4.0, 1.5, 4.0),
            ..Default::default()
        })
        // light
        .spawn(LightComponents {
            translation: Translation::new(4.0, 8.0, 4.0),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(-8.0, 20.0, 15.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}






