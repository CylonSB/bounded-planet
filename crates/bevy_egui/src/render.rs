use bevy::{
    prelude::*,
    render::{
        camera::{
            ActiveCameras,
            Camera,
            OrthographicProjection,
            VisibleEntities,
            WindowOrigin
        },
        pass::{
            LoadOp,
            Operations,
            PassDescriptor,
            RenderPassDepthStencilAttachmentDescriptor,
            TextureAttachment,
        },
        pipeline::*,
        render_graph::{
            base,
            AssetRenderResourcesNode,
            CameraNode,
            NodeId,
            PassNode,
            RenderGraph,
            WindowSwapChainNode,
            WindowTextureNode,
        },
        shader::{
            Shader,
            ShaderStage,
            ShaderStages,
        },
        texture::TextureFormat
    }
};

use crate::egui_node::{
    EguiNode,
    EguiSystemNode
};

#[derive(Bundle)]
pub struct EguiCameraComponents {
    pub camera: Camera,
    pub orthographic_projection: OrthographicProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for EguiCameraComponents {
    fn default() -> Self {
        // we want 0 to be "closest" and +far to be "farthest" in 2d, so we offset
        // the camera's translation by far and use a right handed coordinate system
        let far = 1000.0;
        EguiCameraComponents {
            camera: Camera {
                name: Some(camera::EGUI_CAMERA_NAME.to_string()),
                ..Default::default()
            },
            orthographic_projection: OrthographicProjection {
                far,
                window_origin: WindowOrigin::BottomLeft,
                ..Default::default()
            },
            visible_entities: Default::default(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, far - 0.1)),
            global_transform: Default::default(),
        }
    }
}

pub const EGUI_PIPELINE_HANDLE: Handle<PipelineDescriptor> =
    // No clue where the hell this number comes from, but I changed a few of them to be slightly different
    Handle::from_u128(323432602125399287835112542539754486265);

pub fn build_egui_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        index_format: IndexFormat::Uint16,
        primitive_topology: PrimitiveTopology::TriangleList,

        rasterization_state: Some(RasterizationStateDescriptor {
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            clamp_depth: false,
        }),
        depth_stencil_state: Some(DepthStencilStateDescriptor {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: CompareFunction::Always,
            stencil: StencilStateDescriptor {
                front: StencilStateFaceDescriptor::IGNORE,
                back: StencilStateFaceDescriptor::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
        }),
        color_states: vec![ColorStateDescriptor {
            format: TextureFormat::Bgra8UnormSrgb,
            color_blend: BlendDescriptor {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha_blend: BlendDescriptor {
                src_factor: BlendFactor::OneMinusDstAlpha,
                dst_factor: BlendFactor::One,
                // src_factor: BlendFactor::One,
                // dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            write_mask: ColorWrite::ALL,
        }],
        ..PipelineDescriptor::new(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                include_str!("egui.vert"),
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("egui.frag"),
            ))),
        })
    }
}

pub mod node {
    pub const EGUI_CAMERA_ID: &str = "egui_camera";
    pub const EGUI_SYSTEM_NODE_ID: &str = "egui_system_node";
    pub const EGUI_NODE_ID: &str = "egui_node";
    pub const EGUI_PASS_ID: &str = "egui_pass";
}

pub mod camera {
    pub const EGUI_CAMERA_NAME: &str = "EguiCamera";
}

/// Helper trait only designed to be implemented on [`RenderGraph`], for adding egui stuff to the render graph.
pub trait EguiRenderGraphBuilder {
    /// Add egui nodes and wiring to the [`RenderGraph`].
    fn add_egui_graph(&mut self, resources: &Resources) -> &mut Self;
}

impl EguiRenderGraphBuilder for RenderGraph {
    fn add_egui_graph(&mut self, resources: &Resources) -> &mut Self {
        let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
        let msaa = resources.get::<Msaa>().unwrap();
        pipelines.set(EGUI_PIPELINE_HANDLE, build_egui_pipeline(&mut shaders));

        let mut egui_pass_node = PassNode::<&Handle<EguiNode>>::new(PassDescriptor {
            color_attachments: vec![msaa.color_attachment_descriptor(
                TextureAttachment::Input("color_attachment".to_string()),
                TextureAttachment::Input("color_resolve_target".to_string()),
                Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            )],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
                attachment: TextureAttachment::Input("depth".to_string()),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
            sample_count: msaa.samples,
        });

        egui_pass_node.add_camera(camera::EGUI_CAMERA_NAME);
        self.add_node(node::EGUI_PASS_ID, egui_pass_node);

        self.add_slot_edge(
            base::node::PRIMARY_SWAP_CHAIN,
            WindowSwapChainNode::OUT_TEXTURE,
            node::EGUI_PASS_ID,
            if msaa.samples > 1 {
                "color_resolve_target"
            } else {
                "color_attachment"
            },
        )
        .unwrap();

        self.add_slot_edge(
            base::node::MAIN_DEPTH_TEXTURE,
            WindowTextureNode::OUT_TEXTURE,
            node::EGUI_PASS_ID,
            "depth",
        )
        .unwrap();

        if msaa.samples > 1 {
            self.add_slot_edge(
                base::node::MAIN_SAMPLED_COLOR_ATTACHMENT,
                WindowSwapChainNode::OUT_TEXTURE,
                node::EGUI_PASS_ID,
                "color_attachment",
            )
            .unwrap();
        }

        // ensure ui pass runs after main pass
        self.add_node_edge(base::node::MAIN_PASS, node::EGUI_PASS_ID)
            .unwrap();

        // setup ui camera
        self.add_system_node(node::EGUI_CAMERA_ID, CameraNode::new(camera::EGUI_CAMERA_NAME));
        self.add_node_edge(node::EGUI_CAMERA_ID, node::EGUI_PASS_ID).unwrap();

        let mut active_cameras = resources.get_mut::<ActiveCameras>().unwrap();
        active_cameras.add(camera::EGUI_CAMERA_NAME);

        // Add the egui nodes and wire them up to the egui pass
        self.add_system_node(node::EGUI_NODE_ID, AssetRenderResourcesNode::<EguiNode>::new(false));

        self.add_node_edge(node::EGUI_NODE_ID, node::EGUI_PASS_ID).unwrap();
        
        self
    }
}

/// Extension trait for [`RenderGraph`] that takes an [`EguiSystemNode`] and wires it up to the render graph
/// with an automatically generated unique name, meaning it won't conflict with any other nodes.
pub trait AddEguiSystemNode {
    /// Wires [`EguiSystemNode`] into the render graph with a auto-generated unique name.
    fn add_egui_system_node(&mut self, node: EguiSystemNode) -> NodeId;
}

impl AddEguiSystemNode for RenderGraph {
    fn add_egui_system_node(&mut self, node: EguiSystemNode) -> NodeId {
        let formatted_name = format!("{}_{:?}", node::EGUI_SYSTEM_NODE_ID, node.context.id);

        let id = self.add_system_node(formatted_name.clone(), node);
        self.add_node_edge(formatted_name, node::EGUI_PASS_ID).unwrap();

        id
    }
}