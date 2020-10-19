use std::{convert::TryFrom, fs, path::PathBuf, io::Write};
use bounded_planet::land::{texture_to_mesh_data};
use bounded_planet::land::heightmap::{HeightmapData, SamplingError};
use bounded_planet::land::mesh::MAX_INDEX_COUNT;
use structopt::StructOpt;
use thiserror::Error;
use anyhow;
use image::GrayImage;
use image;
use rmp_serde;
use flate2;
use tracing::{Level, error};

#[derive(StructOpt, Debug)]
#[structopt(name = "server")]
struct Opt {
    /// Path to heightmaps folder or file
    #[structopt(long = "path", required=true)]
    path: PathBuf,
}

#[derive(Debug, Error)]
enum Errors {
    #[error("The provided path was not a png file or folder: `{path:?}`")]
	InvalidPath {
		path: PathBuf
    },

    #[error("The provided heightmap was too large, was {:?} pixels too large. path: {:?}", .0.oversize, .0.path)]
	InputImageTooLarge(InputImageTooLargeError),
}

#[derive(Debug)]
pub struct InputImageTooLargeError {
    path: Option<PathBuf>,
    oversize: usize
}

pub struct ImageHeightmap<'a> {
    pub texture: &'a GrayImage
}

impl<'a> ImageHeightmap<'a> {
    pub fn new(texture: &GrayImage) -> Result<ImageHeightmap, InputImageTooLargeError>
    {
        let size = usize::try_from(texture.width() * texture.height()).expect("word size is less than 32 bit");
        if size > MAX_INDEX_COUNT {
            Err(InputImageTooLargeError {
                path: None,
                oversize: size-MAX_INDEX_COUNT
            })?
        }
        Ok(ImageHeightmap {
            texture: texture
        })
    }
}

impl<'a> HeightmapData for ImageHeightmap<'a>
{
    fn size(&self) -> (u16, u16) {
        (u16::try_from(self.texture.width()-2).expect("Heightmap is too wide"),
        u16::try_from(self.texture.height()-2).expect("Heightmap is too high"))
    }

    fn sample(&self, x: i32, y: i32) -> Result<f32, SamplingError>
    {
        let x = u32::try_from(x+1).map_err(|_e| SamplingError::ReadOutOfBounds())?;
        let y = u32::try_from(y+1).map_err(|_e| SamplingError::ReadOutOfBounds())?;
        Ok(f32::from(self.texture.get_pixel(x, y)[0]))
    }
}

const EXTENSION: &str = "bpmesh";

fn main() -> anyhow::Result<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(Level::DEBUG)
            .finish(),
    )
    .expect("Failed to configure logging");

    let opt = Opt::from_args();
    run(opt)
}

#[tokio::main]
async fn run(options: Opt) -> anyhow::Result<()> {
	let metadata = fs::metadata(&options.path)?;
	if metadata.is_file() {
        let mut output_path = options.path.clone();
        output_path.set_extension(EXTENSION);
        generate_mesh(&options.path, &output_path)?;
	} else if metadata.is_dir() {
        for entry in fs::read_dir(&options.path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let mut output_path = path.clone();
                output_path.set_extension(EXTENSION);
                if let Err(e) = generate_mesh(&path, &output_path) {
                    error!("{}", e);
                }
            }
        }
	} else {
		return Err(Errors::InvalidPath {path: options.path.clone()})?;
    }
    
    Ok(())
}

fn generate_mesh(file_path: &PathBuf, out_path: &PathBuf) -> anyhow::Result<()> {
    if let Some(ext) = file_path.extension() {
        if ext == EXTENSION {
            return Ok(());
        }
    }
    let image = image::open(file_path)?.grayscale().into_luma();
    let heightmap = ImageHeightmap::new(&image).map_err(|mut e| {
        e.path=Some(file_path.clone());
        Errors::InputImageTooLarge(e)
    })?;
    let mesh = texture_to_mesh_data(&heightmap);

    let mut encoder = flate2::write::ZlibEncoder::new(
        std::fs::File::create(out_path)?,
        flate2::Compression::new(5)
    );
    encoder.write_all(&rmp_serde::to_vec(&mesh)?)?;

    Ok(())
}