use bevy::{
    prelude::*,
    render::{
        pipeline::{
            AsVertexBufferDescriptor,
            VertexBufferDescriptors
        },
        render_graph::RenderGraph,
        shader::asset_shader_defs_system,
        stage as render_stage
    }
};

use crate::{
    components::EguiComponents,
    egui_node::EguiNode, 
    egui_ui::{
        EguiUi,
        EguiFrameStartEvent,
        egui_state_update
    },
    mesh_handler::BevyEguiVertex,
    render::{
        EguiCameraComponents,
        EguiRenderGraphBuilder
    },
    systems::{
        EguiContext,
        EguiInput,
        egui_draw_system,
        egui_system_node_adder,
        egui_gather_input
    }
};

#[derive(Debug, Default)]
pub struct EguiPlugin;

impl Plugin for EguiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .init_resource::<EguiUi>()
            .init_resource::<EguiInput>()

            .add_asset::<EguiNode>()
            .add_asset::<EguiContext>()

            .add_event::<EguiFrameStartEvent>()

            // Add this to the POST_UPDATE stage following the convention laid out by existing bevy systems
            .add_system_to_stage(
                stage::POST_UPDATE,
                asset_shader_defs_system::<EguiNode>.system(),
            )
            // Add this to the DRAW stage following the convention laid out by existing bevy systems
            .add_system_to_stage(
                render_stage::DRAW,
                egui_draw_system.system()
            )
            // This needs to be done as soon as possible, so the input is ready for the state update
            .add_system_to_stage_front(
                stage::FIRST,
                egui_gather_input.system()
            )
            // This needs to be performed early so the egui context frames are started for the other systems to use
            .add_system_to_stage(
                stage::FIRST,
                egui_state_update.system()
            )
            .add_system(egui_system_node_adder.system())
            .add_startup_system(setup.thread_local_system());
        
        let resources = app.resources();
        
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_egui_graph(resources);

        let mut nodes = resources.get_mut::<Assets<EguiNode>>().unwrap();
        nodes.add_default(EguiNode::initial_default(resources));

        let mut contexts = resources.get_mut::<Assets<EguiContext>>().unwrap();

        contexts.add(EguiContext::default());
    }
}

fn setup(
    world: &mut World,
    resources: &mut Resources,
) {
    let mut vertex_buffer_descriptors = resources.get_mut::<VertexBufferDescriptors>().unwrap();

    vertex_buffer_descriptors.set(BevyEguiVertex::as_vertex_buffer_descriptor().clone());
    world.spawn(EguiComponents::from_setup(resources));
    world.spawn(EguiCameraComponents::default());
}