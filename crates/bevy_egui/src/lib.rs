use std::sync::Arc;

use bevy::{core::AsBytes, prelude::*, render::draw::DrawContext, render::mesh::INDEX_BUFFER_ASSET_INDEX, render::mesh::VERTEX_BUFFER_ASSET_INDEX, render::pipeline::IndexFormat, render::pipeline::PipelineSpecialization, render::pipeline::PrimitiveTopology, render::pipeline::RenderPipeline, render::pipeline::VertexBufferDescriptor, render::pipeline::VertexBufferDescriptors, render::render_graph::CommandQueue, render::renderer::BufferId, render::renderer::BufferInfo, render::renderer::BufferUsage, render::renderer::RenderResourceBindings, render::renderer::RenderResourceContext, render::renderer::RenderResourceId, render::shader::asset_shader_defs_system, render::shader::shader_defs_system, render::texture::TextureDescriptor, render::texture::TextureFormat};
use bevy::render::render_graph::RenderGraph;
use bevy::render::pipeline::AsVertexBufferDescriptor;
use bevy::app::stage;
use bevy::render::stage as render_stage;

use bevy::render::{
    mesh::{
        VertexAttribute,
        VertexAttributeValues
    },
};

mod base_setup;
use base_setup::{EGUI_PIPELINE_HANDLE, EguiCameraComponents, EguiRenderGraphBuilder};

mod mesh_handler;
mod texture_handler;

mod egui_node;
use egui_node::{EguiComponents, EguiJobsDescriptor, EguiNode};

mod egui_ui;
use egui_ui::{EguiUi, egui_state_update, EguiFrameStartEvent};

#[derive(Debug, Default)]
pub struct EguiPlugin;

// pub mod stage {
//     pub const EGUI: &str = "egui";
// }

impl bevy::prelude::Plugin for EguiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .init_resource::<EguiUi>()
            .add_asset::<EguiNode>()
            .add_event::<EguiFrameStartEvent>()
            .add_system_to_stage(
                stage::POST_UPDATE,
                asset_shader_defs_system::<EguiNode>.system(),
            )
            .add_system_to_stage(stage::FIRST, egui_state_update.system())
            .add_system_to_stage(render_stage::DRAW, egui_draw_system.system())
            .add_startup_system(setup.thread_local_system())

            .add_system(egui_test_system.system());
        
        let resources = app.resources();
        
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph.add_egui_graph(resources);

        let mut nodes = resources.get_mut::<Assets<EguiNode>>().unwrap();
        nodes.add_default(EguiNode::initial_default(resources));
    }
}

fn egui_draw_system(
    mut draw_context: DrawContext,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    msaa: Res<Msaa>,
    mut query: Query<With<Handle<EguiNode>, (&mut Draw, &mut RenderPipelines, &EguiJobsDescriptor)>>,
) {
    for (mut draw, mut render_pipelines, jobs_descriptor) in &mut query.iter() {
        // if !draw.is_visible {
        //     continue;
        // }
        let render_pipelines = &mut *render_pipelines;
        // for pipeline in render_pipelines.pipelines.iter_mut() {
        //     pipeline.specialization.sample_count = msaa.samples;
        // }

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


            if let Some(indices) = indices {
                for (indices, base_vertex) in &jobs_descriptor.jobs {
                    println!("Draw for indices {:?} from base vertex {:?}", indices, base_vertex);
                    draw.draw_indexed(indices.clone(), *base_vertex, 0..1);
                }
                // draw.draw_indexed(indices, 0, 0..1);
            }
        }
    }
}

fn setup(
    world: &mut World,
    resources: &mut Resources,
) {
    println!("Running setup...");
    let mut vertex_buffer_descriptors = resources.get_mut::<VertexBufferDescriptors>().unwrap();

    vertex_buffer_descriptors.set(BevyEguiVertex::as_vertex_buffer_descriptor().clone());
    world.spawn(EguiComponents::from_setup(resources));
    world.spawn(EguiCameraComponents::default());
}

pub fn egui_test_system(
    mut egui: ResMut<EguiUi>,
) {
    let ui = match egui.ui.as_mut() {
        None => return,
        Some(ui) => ui
    };

    let mut value = 1.0;

    println!("Rendering ui stuffs!");

    let window = Window::new("Debug");
    
    window.fixed_pos([500.0, 200.0]).show(ui.ctx(), |ui| {
        ui.label(format!("Hello, world {}", 123));
        ui.add(egui::widgets::Label::new("Hello world!").heading());
        if ui.add(egui::widgets::Button::new("Save").fill(Some(Rgba::RED.into()))).clicked {
            println!("Save clicked!");
        }
        ui.add(Slider::f32(&mut value, 0.0..=1.0).text("float"));
    });
}

use egui::{Rgba, Slider, paint::{Triangles, tessellator::Vertex as EguiVertex}};
use egui::math::Pos2;
use egui::paint::color::Srgba;
use egui::containers::Window;
use mesh_handler::MeshHandler;
// use texture_handler::TextureHandler;

#[repr(C)]
#[derive(Debug, Clone, AsVertexBufferDescriptor)]
struct BevyEguiVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
    clip_min: [f32; 2],
    clip_max: [f32; 2],
}

#[derive(Default)]
struct EguiBuffersBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    clip_mins: Vec<[f32; 2]>,
    clip_maxs: Vec<[f32; 2]>,

    indices: Vec<u32>,

    jobs_descriptor: EguiJobsDescriptor,
}

impl EguiBuffersBuilder {
    pub(crate) fn with_capacity(vertex_capacity: usize, index_capacity: usize) -> Self {
        Self {
            positions: Vec::with_capacity(vertex_capacity),
            normals: Vec::with_capacity(vertex_capacity),
            uvs: Vec::with_capacity(vertex_capacity),
            colors: Vec::with_capacity(vertex_capacity),
            clip_mins: Vec::with_capacity(vertex_capacity),
            clip_maxs: Vec::with_capacity(vertex_capacity),

            indices: Vec::with_capacity(index_capacity),

            jobs_descriptor: Default::default(),
        }
    }

    pub(crate) fn preallocated_from_jobs(jobs: &Vec<(egui::math::Rect, Triangles)>) -> Self {
        let mut jobs_descriptor = EguiJobsDescriptor::default();

        let (num_vertices, num_indices) = jobs.iter()
            .fold((0, 0), |(vertices, indices), (_, triangles)| {
                let (new_vertices, new_indices) = (vertices + triangles.vertices.len(), indices + triangles.indices.len());

                jobs_descriptor.jobs.push(
                    ((indices as _)..(new_indices as _), vertices as _)
                );

                (new_vertices, new_indices)
            });

        println!("Num vertices: {:?}", num_vertices);
        println!("Num indices: {:?}", num_indices);
        println!("Jobs descriptor: {:?}", jobs_descriptor.jobs);

        Self {
            jobs_descriptor,
            ..Self::with_capacity(num_vertices, num_indices)
        }
    }

    pub(crate) fn add_vertices(&mut self, vertices: Vec<EguiVertex>, clip_rect: egui::math::Rect) {
        for EguiVertex {
            pos: Pos2 { x, y },
            uv: Pos2 { x: u, y: v},
            color
        } in vertices {
            // println!("Vertex: {:?}", EguiVertex {
            //     pos: Pos2 { x, y },
            //     uv,
            //     color: Srgba([r, g, b, a])
            // });

            self.positions.push([x, y, 0.0]);
            self.normals.push([0.0, 0.0, 1.0]);
            self.uvs.push([u, v]);
    
            // Converts colors into linear ahead of time
            // let rgba: Rgba = color.into();
            // self.colors.push([
            //     rgba.r(),
            //     rgba.g(),
            //     rgba.b(),
            //     rgba.a(),
            // ]);

            // Just directly represents colors as srgba
            let (r, g, b, a) = color.to_tuple();
            self.colors.push([
                f32::from(r),
                f32::from(g),
                f32::from(b),
                f32::from(a),
            ]);

            let Pos2 { x: min_x, y: min_y } = clip_rect.min;
            let Pos2 { x: max_x, y: max_y } = clip_rect.max;
    
            self.clip_mins.push([min_x, min_y]);
            self.clip_maxs.push([max_x, max_y]);
        }
    }

    pub(crate) fn add_indices(&mut self, indices: &mut Vec<u32>) {
        self.indices.append(indices);
    }

    pub(crate) fn add_from_job(&mut self, (clip_rect, mut triangles): (egui::math::Rect, Triangles)) {
        self.add_indices(&mut triangles.indices);
        self.add_vertices(triangles.vertices, clip_rect);
    }

    pub(crate) fn build_from_jobs(jobs: Vec<(egui::math::Rect, Triangles)>) -> Self {
        println!("Original jobs length: {}", jobs.len());
        let jobs = jobs.into_iter().flat_map(|(rect, triangles)| {
            triangles.split_to_u16().into_iter().map(move |triangles| {
                (rect, triangles)
            })
        }).collect::<Vec<(egui::math::Rect, Triangles)>>();
        println!("Jobs length split to u16: {}", jobs.len());

        let mut builder = Self::preallocated_from_jobs(&jobs);
        jobs.into_iter()
            .for_each(|job| {
                builder.add_from_job(job)
            });

        builder
    }

    pub(crate) fn build(self) -> (Vec<VertexAttribute>, Vec<u32>, EguiJobsDescriptor) {
        (
            vec![
                VertexAttribute {
                    name: "BevyEguiVertex_Position".into(),
                    values: VertexAttributeValues::Float3(self.positions)
                },
                VertexAttribute {
                    name: "BevyEguiVertex_Normal".into(),
                    values: VertexAttributeValues::Float3(self.normals),
                },
                VertexAttribute {
                    name: "BevyEguiVertex_Uv".into(),
                    values: VertexAttributeValues::Float2(self.uvs),
                },
                VertexAttribute {
                    name: "BevyEguiVertex_Color".into(),
                    values: VertexAttributeValues::Float4(self.colors)
                },
                VertexAttribute {
                    name: "BevyEguiVertex_ClipMin".into(),
                    values: VertexAttributeValues::Float2(self.clip_mins)
                },
                VertexAttribute {
                    name: "BevyEguiVertex_ClipMax".into(),
                    values: VertexAttributeValues::Float2(self.clip_maxs)
                }
            ],
            self.indices,
            self.jobs_descriptor
        )
    }
}
