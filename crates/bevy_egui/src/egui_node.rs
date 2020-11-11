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
    components::EguiJobsDescriptor,
    egui_ui::EguiFrameStartEvent,
    systems::EguiInput,
    mesh_handler::MeshHandler,
    systems::EguiContext
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
        let egui_input = resources.get::<EguiInput>().unwrap();

        let mut context = egui::Context::new();

        let _ = context.begin_frame(egui_input.raw_input.clone());
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
    }
}

impl SystemNode for EguiSystemNode {
    fn get_system(&self, commands: &mut Commands) -> Box<dyn System> {
        let system = egui_node_system.system();

        commands.insert_local_resource(
            system.id(),
            EguiSystemNodeState::from_node(self)
        );

        system
    }
}

/// Internal state for the egui system node update system
struct EguiSystemNodeState {
    /// The egui context this system node is associated with.
    context: Handle<EguiContext>,
    /// The entity associated with the egui context which actually enables drawing of the ui.
    draw_entity: Option<Entity>,
    /// The bevy command queue that enables sending render resource manipulation commands.
    command_queue: CommandQueue,
    /// The handler which processes updates to the mesh drawn to.
    mesh_handler: MeshHandler,
}

impl EguiSystemNodeState {
    fn from_node(node: &EguiSystemNode) -> Self {
        Self {
            context: node.context,
            draw_entity: None,
            command_queue: node.command_queue.clone(),
            mesh_handler: MeshHandler::default(),
        }
    }
}

impl FromResources for EguiSystemNodeState {
    fn from_resources(_: &Resources) -> Self {
        // This trait impl only exists because Local<T> requires that T: FromResources, even though
        // in the SystemNode impl for EguiSystemNode above, we manually insert the resource with the value we want.
        // Because of this, this function should never run. If it does, something has changed in bevy and we need to fix this!
        unimplemented!("This is an unneeded implementation and shouldn't ever be run!");
    }
}

/// Update system for an egui system node
#[allow(clippy::type_complexity)]
#[allow(clippy::clippy::too_many_arguments)]
fn egui_node_system(
    mut state: Local<EguiSystemNodeState>,
    egui_input: Res<EguiInput>,
    mut frame_init_event: ResMut<Events<EguiFrameStartEvent>>,

    mut nodes: ResMut<Assets<EguiNode>>,
    mut contexts: ResMut<Assets<EguiContext>>,

    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut textures: ResMut<Assets<Texture>>,

    mut query: Query<(Entity, &mut EguiJobsDescriptor, &mut Handle<EguiNode>, &mut Handle<Mesh>, &mut RenderPipelines)>,
) {
    // Easier to put the annotation then mess up the nice shorthand with underscores on just the bindings
    #[allow(unused_variables)]
    let EguiSystemNodeState {
        context,
        draw_entity,
        command_queue,
        mesh_handler,
    } = &mut *state;

    let render_resource_context = &**render_resource_context;
    let EguiContext { context, .. } = contexts.get_mut(context)
        .expect("Context handle associated with egui node system isn't valid!");

    // Get this context's draw entity, or grab it from the query if we don't yet have it
    let draw_entity = draw_entity.get_or_insert_with(|| {
        let mut query_iter = query.iter();
        let (entity, ..) = query_iter.iter().next().expect("No egui draw entities seem to exist!");
        
        entity
    });

    let mut entity_query = query.entity(*draw_entity)
        .expect("Unable to make query from egui draw entity! This is a logic error indicating it's somehow been removed or other stuff broke!");
    let (_, mut jobs_descriptor, egui_node_handle, mesh_handle, render_pipelines) = entity_query.get()
        .expect("Egui draw entity did not match query! This is a logic error indicating someone has changed the components or removed the entity!");

    if let Some(node) = nodes.get_mut(&egui_node_handle) {
        let egui_texture = context.texture();

        if egui_texture.version != node.texture_hash {
            let tex = textures.get_mut(&node.texture)
                .expect("Egui node texture handle is invalid for texture assets!");

            tex.size = Vec2::new(egui_texture.width as _, egui_texture.height as _);
            tex.data = egui_texture.pixels.clone();

            node.texture_hash = egui_texture.version;
        }
    }

    let (_output, jobs) = context.end_frame();

    // TODO(#54): update mesh handler to use staging buffers via command_queue to make this less bad
    mesh_handler.with_context(render_resource_context, *mesh_handle)
        .update_from_jobs(jobs, &mut jobs_descriptor)
        .set_pipeline_bindings(render_pipelines);

    // TODO(#60) actually give real input to egui, should require the context to actually be owned by the state update system and sent here rather than the other way around
    // Now that frame update crap is over, begin a new frame and send the Ui object away to be used
    frame_init_event.send(EguiFrameStartEvent {
        new_ui: context.begin_frame(egui_input.raw_input.clone())
    });
}