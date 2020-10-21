use std::{borrow::Cow, collections::HashMap, collections::HashSet, sync::Arc};

use bevy::{prelude::*, render::mesh::VertexAttribute, render::mesh::VertexAttributeValues, render::pipeline::DynamicBinding, render::pipeline::PipelineSpecialization, render::pipeline::PrimitiveTopology, render::pipeline::RenderPipeline, render::render_graph::ResourceSlots, render::renderer::RenderContext, render::renderer::RenderResourceContext, render::texture::TextureFormat};
use bevy::render::{
    renderer::RenderResources,
    shader::ShaderDefs,
    render_graph::{
        SystemNode,
        Node,
        CommandQueue
    },
};
use egui::{RawInput, Srgba, Rgba};

use crate::{egui_ui::EguiFrameStartEvent, mesh_handler::MeshHandler};

const FAKE_RAW_INPUT: egui::RawInput = egui::RawInput {
    mouse_down: false,
    mouse_pos: None,
    scroll_delta: egui::math::vec2(0.0, 0.0),
    screen_size: egui::math::vec2(1280.0, 720.0),
    pixels_per_point: Some(1.0),
    time: 0.0,
    events: Vec::new()
};

// TODO: change this to use an Option<Handle<Texture>> with a shader def so we don't have this weird default crap
// TODO: finally split into separate components for the egui rendering passes and the egui render resources
/// Represents the egui rendering passes and systems.
/// Entities with this component are expected to represent rendering to the specified context.
/// This node, with the context name intended for entities to operate on, is also added as a system node.
#[derive(Clone, RenderResources, ShaderDefs)]
pub struct EguiNode {
    pub texture: Handle<Texture>,
    #[render_resources(ignore)]
    pub texture_hash: u64
}

impl EguiNode {
    /// Given an egui context and the texture assets, creates a texture based on the egui context
    pub(crate) fn initial_default(resources: &Resources) -> Self {
        let mut textures = resources.get_mut::<Assets<Texture>>().unwrap();

        let mut context = egui::Context::new();

        let _ = context.begin_frame(FAKE_RAW_INPUT);
        let _ = context.end_frame();

        let egui_texture = context.texture();

        Self {
            texture: textures.add(Texture::new(
                Vec2::new(egui_texture.width as _, egui_texture.height as _),
                egui_texture.pixels.clone(),
                TextureFormat::R8Unorm,
            )),
            texture_hash: egui_texture.version,
        }
    }
}

use std::ops::Range;

#[derive(Default)]
pub struct EguiJobsDescriptor {
    pub jobs: Vec<(Range<u32>, i32)>
}

pub struct EguiId(&'static str);

impl Default for EguiId {
    fn default() -> Self {
        Self("DEFAULT")
    }
}

#[derive(Bundle)]
pub struct EguiComponents {

    pub id: EguiId,
    pub jobs_descriptor: EguiJobsDescriptor,
    /// Associates our entity with the Egui render passes and holds information to associate with an Egui context.
    pub egui_node: Handle<EguiNode>,
    pub mesh: Handle<Mesh>,
    pub render_pipelines: RenderPipelines,
    pub draw: Draw,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for EguiComponents {
    fn default() -> Self {
        Self {
            id: Default::default(),
            jobs_descriptor: Default::default(),
            egui_node: Default::default(),
            mesh: Default::default(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                crate::base_setup::EGUI_PIPELINE_HANDLE,
                PipelineSpecialization {
                    dynamic_bindings: vec![
                        // Transform
                        DynamicBinding {
                            bind_group: 2,
                            binding: 0
                        },
                    ],
                    ..Default::default()
                },
            )]),
            draw: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

impl EguiComponents {
    pub(crate) fn from_setup(resources: &Resources) -> Self {
        let mut meshes = resources.get_mut::<Assets<Mesh>>().unwrap();

        Self {
            // mesh: meshes.add(Mesh::from(shape::Cube::default())),
            mesh: meshes.add(Mesh {
                primitive_topology: PrimitiveTopology::TriangleList,
                attributes: vec![
                    VertexAttribute {
                        name: "BevyEguiVertex_Position".into(),
                        values: VertexAttributeValues::Float3(Vec::new())
                    },
                    VertexAttribute {
                        name: "BevyEguiVertex_Normal".into(),
                        values: VertexAttributeValues::Float3(Vec::new()),
                    },
                    VertexAttribute {
                        name: "BevyEguiVertex_Uv".into(),
                        values: VertexAttributeValues::Float2(Vec::new()),
                    },
                    VertexAttribute {
                        name: "BevyEguiVertex_Color".into(),
                        values: VertexAttributeValues::Float4(Vec::new())
                    },
                    VertexAttribute {
                        name: "BevyEguiVertex_ClipMin".into(),
                        values: VertexAttributeValues::Float2(Vec::new())
                    },
                    VertexAttribute {
                        name: "BevyEguiVertex_ClipMax".into(),
                        values: VertexAttributeValues::Float2(Vec::new())
                    }
                ],
                indices: Some(Vec::new()),
            }),
            ..Default::default()
        }
    }
}

#[derive(Clone, Default)]
pub struct EguiSystemNode {
    /// The egui context this is attached to
    // pub context: Arc<egui::Context>,
    pub command_queue: CommandQueue,
}

impl Node for EguiSystemNode {
    fn update(
        &mut self,
        _world: &World,
        _resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        self.command_queue.execute(render_context);
        println!("Node update!");
    }
}

impl SystemNode for EguiSystemNode {
    fn get_system(&self, commands: &mut Commands) -> Box<dyn System> {
        println!("Grabbing EguiSystemNode system!");
        let system = egui_node_system.system();

        let mut context = egui::Context::new();

        // TODO: provide real RawInput so things aren't possibly bad!
        let _ = context.begin_frame(FAKE_RAW_INPUT);

        commands.insert_local_resource(
            system.id(),
            EguiSystemNodeState {
                context,
                draw_entity: None,

                command_queue: self.command_queue.clone(),

                mesh: None,
                mesh_handler: MeshHandler::default(),
                egui_node: None,
                // texture_handler: TextureHandler::default(),
            }
        );

        system
    }
}

struct EguiSystemNodeState {
    /// The egui context this system node is associated with.
    context: Arc<egui::Context>,
    /// The entity associated with the egui context which actually enables drawing of the ui.
    draw_entity: Option<Entity>,

    /// The bevy command queue that enables sending render resource manipulation commands.
    command_queue: CommandQueue,

    /// Handle to the mesh that is drawn to display this egui.
    mesh: Option<Handle<Mesh>>,
    /// The handler which processes updates to the mesh drawn to.
    mesh_handler: MeshHandler,

    /// Handle to the egui node which stores texture data of this egui.
    egui_node: Option<Handle<EguiNode>>,
    // The Handler which processes updates to the texture of font data.
    // texture_handler: TextureHandler,
}

impl FromResources for EguiSystemNodeState {
    fn from_resources(_: &Resources) -> Self {
        panic!("This is an unneeded implementation and shouldn't ever be run!");
    }
}

// TODO: modify EguiSystemNodeState so that it only holds onto the draw entity and gets the other information each tick from the query
fn egui_node_system(
    mut state: Local<EguiSystemNodeState>,
    mut frame_init_event: ResMut<Events<crate::EguiFrameStartEvent>>,

    mut nodes: ResMut<Assets<EguiNode>>,

    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut textures: ResMut<Assets<Texture>>,

    mut query: Query<(Entity, &mut EguiJobsDescriptor, &mut Handle<EguiNode>, &mut Handle<Mesh>, &mut RenderPipelines)>,
) {
    println!("Egui node system running!");

    // Easier to put the annotation then mess up the nice shorthand with underscores on just the bindings
    #[allow(unused_variables)]
    let EguiSystemNodeState {
        context,
        draw_entity,
        command_queue,
        mesh,
        mesh_handler,
        egui_node,
        // texture_handler,
    } = &mut *state;

    let render_resource_context = &**render_resource_context;

    // If we've not set the draw entity yet, check if there exists the entity.
    // If it doesn't exist, we return and check again next tick.
    if draw_entity.is_none() && query.iter().iter().next().is_none() {
        println!("Leaving early from egui system node update...");
        return;
    }

    // Get this context's draw entity, or grab it from the query if we don't yet have it
    let draw_entity = draw_entity.get_or_insert_with(|| {
        println!("Starting to set draw_entity...");
        let mut query_iter = query.iter();
        let (entity, ..) = query_iter.iter().next().expect("We already checked that the entity exists just above!");
        
        entity
    });

    // Get the egui node on the draw entity, or grab it from the query if we don't yet have it
    let egui_node = egui_node.get_or_insert_with(|| {
        let mut entity_query = query.entity(*draw_entity).unwrap();
        let (_, _, node, ..) = entity_query.get().unwrap();

        *node
    });

    if let Some(node) = nodes.get_mut(egui_node) {
        let egui_texture = context.texture();

        if egui_texture.version != node.texture_hash {
            println!("Texture has changed since first initialization!");
            let tex = textures.get_mut(&node.texture).unwrap();

            tex.size = Vec2::new(egui_texture.width as _, egui_texture.height as _);
            tex.data = egui_texture.pixels.clone();

            node.texture_hash = egui_texture.version;
        } else {
            println!("Texture has _not_ changed since start!");
        }
    }

    // Get the mesh asset to use for rendering this gui, or create it if it doesn't exist
    let mesh = mesh.get_or_insert_with(|| {
        println!("Starting to set mesh...");
        let mut entity_query = query.entity(*draw_entity).unwrap();
        let (_, _, _, mesh, ..) = entity_query.get().unwrap();

        *mesh
    });

    let (_output, jobs) = context.end_frame();

    let mut entity_query = query.entity(*draw_entity)
        .expect("Unable to make query from egui draw entity!");
    let (_, mut jobs_descriptor, node, _, pipelines) = entity_query.get()
        .expect("Egui draw entity did not have a RenderPipelines or EguiJobsDescriptor for some reason?");

    // TODO: update mesh handler to use staging buffers via the command_queue
    mesh_handler.with_context(render_resource_context, *mesh)
        .update_from_jobs(jobs, &mut jobs_descriptor)
        .set_pipeline_bindings(pipelines);

    println!("Trying to send EguiFrameStart event!");

    // TODO: eventually we'll need to give proper input _not_ at the end of the last frame.
    // TODO: to accomplish this, the context should actually be owned by the frame start system and sent here.
    // Now that frame update crap is over, begin a new frame and send the Ui object away to be used
    frame_init_event.send(EguiFrameStartEvent {
        new_ui: context.begin_frame(FAKE_RAW_INPUT)
    });

    println!("Sucessfully sent EguiFrameStart event!");
}