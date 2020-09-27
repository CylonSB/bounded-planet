use bevy::{prelude::Texture, render::texture::TextureFormat};

#[derive(Debug)]
pub enum SamplingError {
    ReadOutOfBounds()
}

pub trait HeightmapData
{
    /// Get the bounds of this heightmap (x, y).
    /// Sampled values must be in the [0, size-1] range.
    fn size(&self) -> (u32, u32);

    /// Sample a height from the heightmap.
    fn sample(&self, x:u32, y:u32) -> Result<f32, SamplingError>;
}

/// Wrap a texture as a heightmap
pub struct TextureHeightmap<'a> {
    pub texture: &'a Texture,
    size: (u32, u32),

    pixel_bytes: u8,
    chan_start: usize,
    chan_end: usize,
}

#[derive(Debug)]
pub enum WrapError {
    UnsupportedFormat(TextureFormat),
}

impl<'a> TextureHeightmap<'a> {
    pub fn new(texture: &Texture) -> Result<TextureHeightmap, WrapError>
    {
        // For now we only support 1 single texture format
        if texture.format != TextureFormat::R8Unorm {
            return Err(WrapError::UnsupportedFormat(texture.format));
        }

        return Ok(TextureHeightmap {
            texture,
            size: (texture.size.x() as u32, texture.size.y() as u32),

            // Because we only support 1 format these are constants
            pixel_bytes: 1,
            chan_start: 0,
            chan_end: 0
        });
    }
}

impl<'a> HeightmapData for TextureHeightmap<'a>
{
    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn sample(&self, x:u32, y:u32) -> Result<f32, SamplingError>
    {
        // Sanity check that read coordinates are in bounds
        if x >= self.size.0 || y > self.size.1 {
            return Err(SamplingError::ReadOutOfBounds())
        }

        // Work of the coordinate in the data array of the bytes for this pixel
        let i = x * (self.pixel_bytes as u32)                // Offset by columns
              + y * (self.pixel_bytes as u32) * self.size.1; // Offset by rows

        // Currently we only support one texture format (R8UNORM), so implementing this very simple...

        // Get the byte
        let data = self.texture.data[i as usize] as f32;

        // Remap it to the right range
        return Ok(data);
    }
}