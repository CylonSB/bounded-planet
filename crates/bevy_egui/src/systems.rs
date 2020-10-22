use std::sync::Arc;

use bevy::{prelude::*, render::{draw::DrawContext, render_graph::RenderGraph, renderer::RenderResourceBindings}};

use egui::{Rgba, Slider, containers::Window};

use crate::{render::AddEguiSystemNode, components::EguiJobsDescriptor, egui_node::EguiNode, egui_node::{EguiSystemNode, FAKE_RAW_INPUT}, egui_ui::EguiUi};

pub fn egui_test_system(
    mut egui: ResMut<EguiUi>,
) {
    let ui = match egui.ui.as_mut() {
        None => return,
        Some(ui) => ui
    };

    let mut value = 1.0;

    println!("Rendering ui stuffs!");

    Window::new("Debug").fixed_pos([500.0, 200.0]).show(ui.ctx(), |ui| {
        ui.label(format!("Hello, world {}", 123));
        ui.add(egui::widgets::Label::new("Hello world!").heading());
        if ui.add(egui::widgets::Button::new("Save").fill(Some(Rgba::RED.into()))).clicked {
            println!("Save clicked!");
        }
        ui.add(Slider::f32(&mut value, 0.0..=1.0).text("float"));
    });
}

#[allow(clippy::type_complexity)]
pub fn egui_draw_system(
    mut draw_context: DrawContext,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    msaa: Res<Msaa>,
    mut query: Query<With<Handle<EguiNode>, (&mut Draw, &mut RenderPipelines, &EguiJobsDescriptor)>>,
) {
    for (mut draw, mut render_pipelines, jobs_descriptor) in &mut query.iter() {
        if !draw.is_visible {
            continue;
        }

        let render_pipelines = &mut *render_pipelines;
        for pipeline in render_pipelines.pipelines.iter_mut() {
            pipeline.specialization.sample_count = msaa.samples;
        }

        // This is needed since draw operations were already done for entity (and it cannot currently be prevented). In fact...
        // TODO(#55): stop the entity from having its draw operations already done, allowing this to be removed
        draw.clear_render_commands();

        for render_pipeline in render_pipelines.pipelines.iter() {
            draw_context
                .set_pipeline(
                    &mut draw,
                    render_pipeline.pipeline,
                    &render_pipeline.specialization,
                )
                .unwrap();
            draw_context
                .set_bind_groups_from_bindings(
                    &mut draw,
                    &mut [
                        &mut render_pipelines.bindings,
                        &mut render_resource_bindings,
                    ],
                )
                .unwrap();
            let indices = draw_context
                .set_vertex_buffers_from_bindings(&mut draw, &[&render_pipelines.bindings])
                .unwrap();


            if indices.is_some() {
                for (indices, base_vertex) in &jobs_descriptor.jobs {
                    draw.draw_indexed(indices.clone(), *base_vertex, 0..1);
                }
            }
        }
    }
}

// TODO(#56): properly integrate the context as an asset to remove the singleton restriction on egui contexts
pub struct EguiContext {
    // TODO(#56): Utilize name as a key for properly integrated context asset
    // name: &'static str,
    pub context: Arc<egui::Context>,
}

impl Default for EguiContext {
    fn default() -> Self {
        Self {
            context: egui::Context::new(),
        }
    }
}

#[derive(Default)]
pub struct EguiSystemNodeAdderState {
    event_reader: EventReader<AssetEvent<EguiContext>>,
}

// TODO(#56): properly integrate the context as an asset to remove the singleton restriction on egui contexts
pub fn egui_system_node_adder(
    mut state: Local<EguiSystemNodeAdderState>,
    
    context_events: Res<Events<AssetEvent<EguiContext>>>,
    mut contexts: ResMut<Assets<EguiContext>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    for event in state.event_reader.iter(&context_events) {
        match event {
            AssetEvent::Created { handle } => {
                let EguiContext { context, .. } = contexts.get_mut(handle).unwrap();

                // Begin frame so that the system is in the correct state
                context.begin_frame(FAKE_RAW_INPUT);

                render_graph.add_egui_system_node(EguiSystemNode {
                    command_queue: Default::default(),
                    context: *handle,
                });
            },
            AssetEvent::Modified { .. } => {
                // TODO(#56): Determine what (if anything?) should happen if there is a modification
            },
            AssetEvent::Removed { .. } => {
                todo!("TODO(#56): Deal with removing the render node stuff for a given egui context!")
            }
        }
    }
}