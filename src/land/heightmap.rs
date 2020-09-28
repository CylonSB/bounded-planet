use bevy::{prelude::Texture, render::texture::TextureFormat};

#[derive(Debug)]
pub enum SamplingError {
    ReadOutOfBounds()
}

pub trait HeightmapData
{
    /// Get the bounds of this heightmap (x, y).
    /// Sampled values must be in the [0, size-1] range.
    fn size(&self) -> (u16, u16);

    /// Sample a height from the heightmap. This allows reads one either side of the size, i.e. `-1` and `size().0` are valid sample positions
    fn sample(&self, x: i32, y: i32) -> Result<f32, SamplingError>;
}

/// Wrap a texture as a heightmap
pub struct TextureHeightmap<'a> {
    pub texture: &'a Texture,
    size: (u16, u16),
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

        Ok(TextureHeightmap {
            texture,
            size: (texture.size.x() as u16 - 2, texture.size.y() as u16 - 2),
        })
    }
}

impl<'a> HeightmapData for TextureHeightmap<'a>
{
    fn size(&self) -> (u16, u16) {
        self.size
    }

    fn sample(&self, x: i32, y: i32) -> Result<f32, SamplingError>
    {
        // Sanity check that read coordinates are in bounds
        if (x > i32::from(self.size.0)) || (y > i32::from(self.size.1)) || (y < -1) || (x < -1) {
            return Err(SamplingError::ReadOutOfBounds())
        }

        // Pull the read back into the valid bounds
        let x = x + 1;
        let y = y + 1;

        // Work of the coordinate in the data array of the bytes for this pixel
        let i = x                               // Offset by columns
              + y * (i32::from(self.size.1) + 2);   // Offset by rows

        // Get the byte
        Ok(f32::from(self.texture.data[i as usize]))
    }
}
