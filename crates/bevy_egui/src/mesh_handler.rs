use bevy::{prelude::*, render::mesh::INDEX_BUFFER_ASSET_INDEX, render::mesh::VERTEX_BUFFER_ASSET_INDEX, render::mesh::VertexAttribute, render::pipeline::VertexBufferDescriptor, render::renderer::BufferId, render::renderer::BufferInfo, render::renderer::BufferUsage, render::renderer::RenderResourceId};
use bevy::{core::AsBytes, render::{pipeline::IndexFormat, renderer::RenderResourceContext}};
use bevy::render::pipeline::AsVertexBufferDescriptor;
use egui::paint::Triangles;
use egui::math::Rect;

use crate::{BevyEguiVertex, EguiBuffersBuilder, egui_node::EguiJobsDescriptor};

pub(crate) struct MeshHandler {
    pub vertex_buffer_descriptor: &'static VertexBufferDescriptor,
    vertex_buffer: Option<BufferId>,
    vertex_bytes: Vec<u8>,

    staging_buffer: Option<BufferId>,
}

impl Default for MeshHandler {
    fn default() -> Self {
        MeshHandler {
            vertex_buffer_descriptor: BevyEguiVertex::as_vertex_buffer_descriptor(),
            staging_buffer: None,
            vertex_buffer: None,
            vertex_bytes: Vec::default(),
        }
    }
}

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
    pub(crate) fn with_context<'a>(&'a mut self, context: &'a dyn RenderResourceContext, mesh: Handle<Mesh>) -> MeshHandlerWithContext<'a> {
        MeshHandlerWithContext {
            parent: self,
            context,
            mesh,
        }
    }

    pub fn update_bytes_from_attributes(
        &mut self,
        attributes: Vec<VertexAttribute>,
        fill_missing_attributes: bool,
    ) {
        // println!("Vertex colors: {:?}", attributes[3]);
        // println!("Vertex attributes: {:?}", attributes);
        // println!("Vertex uvs: {:?}", attributes[2]);
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
                        println!("Missing vertex attribute: {}", vertex_attribute.name);
                    }
                }
            }
        }
    }
}

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
    pub fn update_from_jobs(
        &mut self,
        jobs: Vec<(Rect, Triangles)>,
        jobs_descriptor: &mut EguiJobsDescriptor
    ) -> &mut Self {
        self.remove_current_mesh_resources(self.mesh);

        let buffers_builder = EguiBuffersBuilder::build_from_jobs(jobs);
        let (attributes, indices, descriptor) = buffers_builder.build();

        *jobs_descriptor = descriptor;
        // println!("Vertex attributes: {:?}", attributes);

        let vertex_buffer = self.make_vertex_buffer(attributes);
        let index_buffer = self.make_index_buffer(indices);

        self.context.set_asset_resource(
            self.mesh,
            RenderResourceId::Buffer(vertex_buffer),
            VERTEX_BUFFER_ASSET_INDEX,
        );

        self.context.set_asset_resource(
            self.mesh,
            RenderResourceId::Buffer(index_buffer),
            INDEX_BUFFER_ASSET_INDEX,
        );

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

    fn make_index_buffer(&self, indices: Vec<u32>) -> BufferId {
        let index_bytes = indices_as_bytes(&indices, IndexFormat::Uint16);
        // let index_bytes = indices_as_bytes(&indices, IndexFormat::Uint32);
        self.context.create_buffer_with_data(
            BufferInfo {
                buffer_usage: BufferUsage::INDEX,
                ..Default::default()
            },
            &index_bytes
        )
    }

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