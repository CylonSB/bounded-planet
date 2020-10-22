use bevy::{
    prelude::*,
    render::{
        shader::ShaderDefs,
        texture::TextureFormat,
        render_graph::{
            Node,
            SystemNode,
            CommandQueue,
            ResourceSlots,
        },
        renderer::{
            RenderContext,
            RenderResourceContext,
            RenderResources
        }
    }
};

use crate::{
    egui_ui::EguiFrameStartEvent,
    mesh_handler::MeshHandler,
    components::EguiJobsDescriptor,
    systems::EguiContext
};

// TODO(#60) actually give real input to egui
/// Default fake input so the gui can at least work.
pub const FAKE_RAW_INPUT: egui::RawInput = egui::RawInput {
    mouse_down: false,
    mouse_pos: None,
    scroll_delta: egui::math::vec2(0.0, 0.0),
    screen_size: egui::math::vec2(1280.0, 720.0),
    pixels_per_point: Some(1.0),
    time: 0.0,
    events: Vec::new()
};

// TODO(#57): finally split into separate components for the egui rendering passes and the egui render resources
// TODO(#58) change this to use an Option<Handle<Texture>> with a shader def so we don't have this weird default crap
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

/// Render system node to do render utility updates for a given egui context
#[derive(Clone)]
pub struct EguiSystemNode {
    /// The egui context this is attached to
    pub context: Handle<EguiContext>,
    /// Command queue to assist with render resource operations
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

        commands.insert_local_resource(
            system.id(),
            EguiSystemNodeState::from_node(self)
        );

        system
    }
}

// TODO(#59) modify EguiSystemNodeState so that it only holds onto the draw entity and gets the other information each tick from the query
/// Internal state for the egui system node update system
struct EguiSystemNodeState {
    /// The egui context this system node is associated with.
    context: Handle<EguiContext>,
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
}

impl EguiSystemNodeState {
    fn from_node(node: &EguiSystemNode) -> Self {
        Self {
            context: node.context,
            draw_entity: None,
            command_queue: node.command_queue.clone(),
            mesh: None,
            mesh_handler: MeshHandler::default(),
            egui_node: None,
        }
    }
}

impl FromResources for EguiSystemNodeState {
    fn from_resources(_: &Resources) -> Self {
        panic!("This is an unneeded implementation and shouldn't ever be run!");
    }
}

/// Update system for a egui system node
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn egui_node_system(
    mut state: Local<EguiSystemNodeState>,
    mut frame_init_event: ResMut<Events<EguiFrameStartEvent>>,

    mut nodes: ResMut<Assets<EguiNode>>,
    mut contexts: ResMut<Assets<EguiContext>>,

    render_resource_context: Res<Box<dyn RenderResourceContext>>,
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

    let EguiContext { context, .. } = contexts.get_mut(context).unwrap();

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
    let (_, mut jobs_descriptor, _, _, pipelines) = entity_query.get()
        .expect("Egui draw entity did not have a RenderPipelines or EguiJobsDescriptor for some reason?");

    // TODO(#54): update mesh handler to use staging buffers via command_queue to make this less bad
    mesh_handler.with_context(render_resource_context, *mesh)
        .update_from_jobs(jobs, &mut jobs_descriptor)
        .set_pipeline_bindings(pipelines);

    println!("Trying to send EguiFrameStart event!");

    // TODO(#60) actually give real input to egui, should require the context to actually be owned by the state update system and sent here rather than the other way around
    // Now that frame update crap is over, begin a new frame and send the Ui object away to be used
    frame_init_event.send(EguiFrameStartEvent {
        new_ui: context.begin_frame(FAKE_RAW_INPUT)
    });

    println!("Sucessfully sent EguiFrameStart event!");
}