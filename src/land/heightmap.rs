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

    /// Sample a height from the heightmap. This allows reads one either side of the size, i.e. `-1` and `size().0` are valid sample positions
    fn sample(&self, x:i32, y:i32) -> Result<f32, SamplingError>;
}

/// Wrap a texture as a heightmap
pub struct TextureHeightmap<'a> {
    pub texture: &'a Texture,
    size: (u32, u32),
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
            size: (texture.size.x() as u32 - 2, texture.size.y() as u32 - 2),
        });
    }
}

impl<'a> HeightmapData for TextureHeightmap<'a>
{
    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn sample(&self, x:i32, y:i32) -> Result<f32, SamplingError>
    {
        // Sanity check that read coordinates are in bounds
        if (x >= self.size.0 as i32 + 1) || (y > self.size.1 as i32 + 1) || (y < -1) || (x < -1) {
            return Err(SamplingError::ReadOutOfBounds())
        }

        // Pull the read back into the valid bounds
        let x = x + 1;
        let y = y + 1;

        // Work of the coordinate in the data array of the bytes for this pixel
        let i = x                               // Offset by columns
              + y * (self.size.1 as i32 + 2);   // Offset by rows

        // Get the byte
        Ok(self.texture.data[i as usize] as f32)
    }
}