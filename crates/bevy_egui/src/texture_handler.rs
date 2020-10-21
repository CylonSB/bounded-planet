use bevy::{prelude::*, render::renderer::BufferId, render::renderer::RenderResourceContext, render::texture::TextureDescriptor};

pub(crate) struct TextureHandler {
    texture_hash: u64,
}

impl Default for TextureHandler {
    fn default() -> Self {
        TextureHandler {
            texture_hash: 0,
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