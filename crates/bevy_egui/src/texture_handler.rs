use bevy::{prelude::*, render::renderer::BufferId, render::renderer::RenderResourceContext, render::texture::TextureDescriptor};

pub(crate) struct TextureHandler {
    texture_descriptor: TextureDescriptor,
    texture_buffer: Option<BufferId>,
    current_texture_hash: u64,
}

impl Default for TextureHandler {
    fn default() -> Self {
        TextureHandler {
            texture_descriptor: TextureDescriptor::default(),
            texture_buffer: None,
            current_texture_hash: 0,
        }
    }
}

pub(crate) struct TextureHandlerWithContext<'a> {
    parent: &'a mut TextureHandler,
    context: &'a dyn RenderResourceContext,
}

impl std::ops::Deref for TextureHandlerWithContext<'_> {
    type Target = TextureHandler;

    fn deref(&self) -> &Self::Target {
        self.parent
    }
}

impl std::ops::DerefMut for TextureHandlerWithContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.parent
    }
}

pub const ALIGNMENT: usize = 256;
fn get_aligned(data_size: f32) -> usize {
    ALIGNMENT * ((data_size / ALIGNMENT as f32).ceil() as usize)
}

fn update_texture() {
    let texture_descriptor: TextureDescriptor = texture.into();
    let width = texture.size.x() as usize;
    let aligned_width = get_aligned(texture.size.x());
    let format_size = texture.format.pixel_size();
    let mut aligned_data =
        vec![0; format_size * aligned_width * texture.size.y() as usize];
    texture
        .data
        .chunks_exact(format_size * width)
        .enumerate()
        .for_each(|(index, row)| {
            let offset = index * aligned_width * format_size;
            aligned_data[offset..(offset + width * format_size)]
                .copy_from_slice(row);
        });
    let texture_buffer = render_context.resources().create_buffer_with_data(
        BufferInfo {
            buffer_usage: BufferUsage::COPY_SRC,
            ..Default::default()
        },
        &aligned_data,
    );

    let texture_resource = render_context
        .resources()
        .get_asset_resource(*handle, TEXTURE_ASSET_INDEX)
        .unwrap();

    render_context.copy_buffer_to_texture(
        texture_buffer,
        0,
        (format_size * aligned_width) as u32,
        texture_resource.get_texture().unwrap(),
        [0, 0, 0],
        0,
        texture_descriptor.size,
    );
    render_context.resources().remove_buffer(texture_buffer);
}