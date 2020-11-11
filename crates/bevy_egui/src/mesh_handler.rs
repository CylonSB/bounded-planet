use bevy::{
    prelude::*,
    core::AsBytes,
    render::{
        pipeline::{
            VertexBufferDescriptor,
            AsVertexBufferDescriptor,
            IndexFormat
        },
        renderer::{
            BufferId,
            BufferInfo,
            BufferUsage,
            RenderResourceId,
            RenderResourceContext
        },
        mesh::{
            INDEX_BUFFER_ASSET_INDEX,
            VERTEX_BUFFER_ASSET_INDEX,
            VertexAttribute,
            VertexAttributeValues
        },
    }
};

use egui::{
    math::{
        Pos2,
        Rect
    },
    paint::{
        Triangles,
        tessellator::Vertex as EguiVertex
    }
};

use tracing::error;

use crate::components::EguiJobsDescriptor;

/// Handles all operations for updating a mesh based on egui job information.
/// 
/// You must call [`MeshHandler::with_context`] to wrap this into a [`MeshHandlerWithContext`] to unlock main operations.
pub(crate) struct MeshHandler {
    pub vertex_buffer_descriptor: &'static VertexBufferDescriptor,
    vertex_bytes: Vec<u8>,

    // TODO(#54): utilize a staging buffer to make this more better
    // vertex_buffer: Option<BufferId>,
    // staging_buffer: Option<BufferId>,
}

impl Default for MeshHandler {
    fn default() -> Self {
        MeshHandler {
            vertex_buffer_descriptor: BevyEguiVertex::as_vertex_buffer_descriptor(),
            vertex_bytes: Vec::default(),

            // TODO(#54): utilize a staging buffer to make this more better
            // staging_buffer: None,
            // vertex_buffer: None,
        }
    }
}

/// A wrapped RAII [`MeshHandler`] with invocation information to enable the primary operations.
pub(crate) struct MeshHandlerWithContext<'a> {
    parent: &'a mut MeshHandler,
    context: &'a dyn RenderResourceContext,
    mesh: Handle<Mesh>,
}

impl std::ops::Deref for MeshHandlerWithContext<'_> {
    type Target = MeshHandler;

    fn deref(&self) -> &Self::Target {
        self.parent
    }
}

impl std::ops::DerefMut for MeshHandlerWithContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.parent
    }
}

impl MeshHandler {
    /// With a render resource context and a mesh handle to manage, wraps the mesh handler to enable its main operations.
    pub(crate) fn with_context<'a>(&'a mut self, context: &'a dyn RenderResourceContext, mesh: Handle<Mesh>) -> MeshHandlerWithContext<'a> {
        MeshHandlerWithContext {
            parent: self,
            context,
            mesh,
        }
    }

    /// Updates the vertex bytes based on a set of vertex attributes.
    pub(crate) fn update_bytes_from_attributes(
        &mut self,
        attributes: Vec<VertexAttribute>,
        fill_missing_attributes: bool,
    ) {
        let length = attributes.first().map(|a| a.values.len()).unwrap_or(0);
        self.vertex_bytes.clear();
        self.vertex_bytes.resize(self.vertex_buffer_descriptor.stride as usize * length, 0);

        for vertex_attribute in self.vertex_buffer_descriptor.attributes.iter() {
            match attributes
                .iter()
                .find(|a| vertex_attribute.name == a.name)
            {
                Some(mesh_attribute) => {
                    let attribute_bytes = mesh_attribute.values.get_bytes();
                    let attribute_size = vertex_attribute.format.get_size() as usize;
                    for (i, vertex_slice) in attribute_bytes.chunks(attribute_size).enumerate() {
                        let vertex_offset = self.vertex_buffer_descriptor.stride as usize * i;
                        let attribute_offset = vertex_offset + vertex_attribute.offset as usize;
                        self.vertex_bytes[attribute_offset..attribute_offset + attribute_size]
                            .copy_from_slice(vertex_slice);
                    }
                }
                None => {
                    if !fill_missing_attributes {
                        error!("Missing vertex attribute: {}", vertex_attribute.name);
                    }
                }
            }
        }
    }
}

/// Converts the indices into bytes using the given index format.
fn indices_as_bytes(indices: &[u32], index_format: IndexFormat) -> Vec<u8> {
    match index_format {
        IndexFormat::Uint16 => indices
            .iter()
            .map(|i| *i as u16)
            .collect::<Vec<u16>>()
            .as_slice()
            .as_bytes()
            .to_vec(),
        IndexFormat::Uint32 => indices.as_bytes().to_vec(),
    }
}

impl MeshHandlerWithContext<'_> {
    /// Uses the provided job information to update the currently managed mesh.
    pub fn update_from_jobs(
        &mut self,
        jobs: Vec<(Rect, Triangles)>,
        jobs_descriptor: &mut EguiJobsDescriptor
    ) -> &mut Self {
        // Remove all set resources associated with the managed mesh
        self.remove_current_mesh_resources(self.mesh);

        // Using the input job information, build it into buffers
        let buffers_builder = EguiBuffersBuilder::build_from_jobs(jobs);
        let (attributes, indices, descriptor) = buffers_builder.build();

        // Update the input jobs descriptor with the one generated from the input jobs
        *jobs_descriptor = descriptor;

        // Make a vertex and index buffer in the render system based on the built buffer data
        let vertex_buffer = self.make_vertex_buffer(attributes);
        let index_buffer = self.make_index_buffer(indices);

        // Updates the vertex buffer of the managed mesh
        self.context.set_asset_resource(
            self.mesh,
            RenderResourceId::Buffer(vertex_buffer),
            VERTEX_BUFFER_ASSET_INDEX,
        );

        // Updates the index buffer of the managed mesh
        self.context.set_asset_resource(
            self.mesh,
            RenderResourceId::Buffer(index_buffer),
            INDEX_BUFFER_ASSET_INDEX,
        );

        // TODO(#54): utilize a staging buffer to make this more better
        // let staging_buffer = if let Some(staging_buffer) = state.staging_buffer {
        //     render_resource_context.map_buffer(staging_buffer);
        //     staging_buffer
        // } else {
        //     let staging_buffer = render_resource_context.create_buffer_with_data(
        //         BufferInfo {
        //             size: state.buffer_capacity,
        //             buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
        //             mapped_at_creation: true,
        //         },
        //         &vertex_bytes
        //     );

        //     todo!()
        // };

        self
    }

    /// Sets the current mesh information as a binding for the provided render pipelines.
    pub fn set_pipeline_bindings(&mut self, mut pipelines: Mut<RenderPipelines>) -> &mut Self {
        if let Some(RenderResourceId::Buffer(vertex_buffer)) =
            self.context.get_asset_resource(self.mesh, VERTEX_BUFFER_ASSET_INDEX)
        {
            pipelines.bindings.set_vertex_buffer(
                "BevyEguiVertex",
                vertex_buffer,
                self.context
                    .get_asset_resource(self.mesh, INDEX_BUFFER_ASSET_INDEX)
                    .and_then(|r| {
                        if let RenderResourceId::Buffer(buffer) = r {
                            Some(buffer)
                        } else {
                            None
                        }
                    }),
            );
        }

        self
    }

    /// Creates a vertex buffer filled with the provided vertex attributes.
    fn make_vertex_buffer(&mut self, attributes: Vec<VertexAttribute>) -> BufferId {
        self.update_bytes_from_attributes(attributes, false);

        self.context.create_buffer_with_data(
            BufferInfo {
                buffer_usage: BufferUsage::VERTEX,
                ..Default::default()
            },
            &self.vertex_bytes
        )
    }

    /// Creates an index buffer filled with the provided indices.
    fn make_index_buffer(&self, indices: Vec<u32>) -> BufferId {
        // TODO(#53): investigate changing this to u32 indices after that is possible in bevy
        let index_bytes = indices_as_bytes(&indices, IndexFormat::Uint16);

        self.context.create_buffer_with_data(
            BufferInfo {
                buffer_usage: BufferUsage::INDEX,
                ..Default::default()
            },
            &index_bytes
        )
    }

    /// Removes the current render resources associated with the provided mesh handle.
    fn remove_current_mesh_resources(
        &self,
        handle: Handle<Mesh>,
    ) {
        if let Some(RenderResourceId::Buffer(buffer)) =
            self.context.get_asset_resource(handle, VERTEX_BUFFER_ASSET_INDEX)
        {
            self.context.remove_buffer(buffer);
            self.context.remove_asset_resource(handle, VERTEX_BUFFER_ASSET_INDEX);
        }
        if let Some(RenderResourceId::Buffer(buffer)) =
            self.context.get_asset_resource(handle, INDEX_BUFFER_ASSET_INDEX)
        {
            self.context.remove_buffer(buffer);
            self.context.remove_asset_resource(handle, INDEX_BUFFER_ASSET_INDEX);
        }
    }
}



/// Represents the structure of the egui vertex passed into bevy for rendering.
/// Mostly used for providing the vertex buffer descriptor for the shader and render operations.
#[repr(C)]
#[derive(Debug, Clone, AsVertexBufferDescriptor)]
pub struct BevyEguiVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
    clip_min: [f32; 2],
    clip_max: [f32; 2],
}

/// Builds the vertex and index buffers, along with tracking the jobs into the jobs descriptor
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
    /// Creates an [`EguiBuffersBuilder`] with a given capacity for the vertex and index buffers.
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

    /// Given a slice of jobs, calculates the needed capacity for the buffers and creates an [`EguiBuffersBuilder`] with with that capacity
    pub(crate) fn preallocated_from_jobs(jobs: &[(egui::math::Rect, Triangles)]) -> Self {
        let mut jobs_descriptor = EguiJobsDescriptor::default();

        let (num_vertices, num_indices) = jobs.iter()
            .fold((0, 0), |(vertices, indices), (_, triangles)| {
                let (new_vertices, new_indices) = (vertices + triangles.vertices.len(), indices + triangles.indices.len());

                jobs_descriptor.jobs.push(
                    ((indices as _)..(new_indices as _), vertices as _)
                );

                (new_vertices, new_indices)
            });

        Self {
            jobs_descriptor,
            ..Self::with_capacity(num_vertices, num_indices)
        }
    }

    /// Adds a set of vertices with their clip rectangle into the buffer.
    pub(crate) fn add_vertices(&mut self, vertices: Vec<EguiVertex>, clip_rect: egui::math::Rect) {
        for EguiVertex {
            pos: Pos2 { x, y },
            uv: Pos2 { x: u, y: v},
            color
        } in vertices {

            self.positions.push([x, y, 0.0]);
            self.normals.push([0.0, 0.0, 1.0]);
            self.uvs.push([u, v]);
    
            // Just directly represents colors as srgba 0 - 255
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

    /// Adds a set of indices into the buffer.
    pub(crate) fn add_indices(&mut self, mut indices: Vec<u32>) {
        self.indices.append(&mut indices);
    }

    /// From a single egui job (tuple of clip rect and triangles), adds the represented indices and vertices to the buffers.
    pub(crate) fn add_from_job(&mut self, (clip_rect, triangles): (egui::math::Rect, Triangles)) {
        self.add_indices(triangles.indices);
        self.add_vertices(triangles.vertices, clip_rect);
    }

    /// Given a set of egui jobs, creates an [`EguiBuffersBuilder`] and fills it with the jobs.
    pub(crate) fn build_from_jobs(jobs: Vec<(egui::math::Rect, Triangles)>) -> Self {
        let jobs = jobs.into_iter().flat_map(|(rect, triangles)| {
            // TODO(#53): investigate changing this to u32 indices after that is possible in bevy
            triangles.split_to_u16().into_iter().map(move |triangles| {
                (rect, triangles)
            })
        }).collect::<Vec<(egui::math::Rect, Triangles)>>();

        let mut builder = Self::preallocated_from_jobs(&jobs);
        jobs.into_iter()
            .for_each(|job| {
                builder.add_from_job(job)
            });

        builder
    }

    /// Consumes the [`EguiBuffersBuilder`] and outputs the buffers and jobs descriptor.
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