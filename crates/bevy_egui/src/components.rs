use std::ops::Range;

use bevy::{
    prelude::*,
    render::{
        mesh::{
            VertexAttribute,
            VertexAttributeValues
        },
        pipeline::{
            DynamicBinding,
            PipelineSpecialization,
            PrimitiveTopology,
            RenderPipeline
        },
    }
};

use crate::egui_node::EguiNode;

/// Describes the egui jobs with tuples of (index_range, start_vertex)
#[derive(Default)]
pub struct EguiJobsDescriptor {
    pub jobs: Vec<(Range<u32>, i32)>
}

/// Bundle for the draw entity of an egui context. Each context needs a unique draw entity so it can be displayed.
#[derive(Bundle)]
pub struct EguiComponents {
    /// Stores the jobs information necessary to properly batch the mesh draw calls
    pub jobs_descriptor: EguiJobsDescriptor,
    /// Associates our entity with the Egui render passes and holds information to associate with an Egui context.
    pub egui_node: Handle<EguiNode>,
    /// Handle to the actual mesh information for rendering the ui
    pub mesh: Handle<Mesh>,
    /// The render pipelines hooked up to the egui pipeline ensuring proper rendering
    pub render_pipelines: RenderPipelines,
    /// Utility component allowing entity to be drawn
    pub draw: Draw,
    /// Relative spacial location of the gui. Does influence where the ui is drawn, as the ui is still seen through a camera
    pub transform: Transform,
    /// Global spacial location of the gui. Similar to the regular Transform field
    pub global_transform: GlobalTransform,
}

impl Default for EguiComponents {
    fn default() -> Self {
        Self {
            jobs_descriptor: Default::default(),
            egui_node: Default::default(),
            mesh: Default::default(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                crate::render::EGUI_PIPELINE_HANDLE,
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