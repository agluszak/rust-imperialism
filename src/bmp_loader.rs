//! Custom BMP asset loader that handles Imperialism's transparency color
//!
//! This loader processes BMP files from the original game and converts
//! the magenta transparency color RGB(225, 61, 246) to actual transparency.

use bevy::{
    asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader},
    image::Image,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use thiserror::Error;

/// RGB color that should be treated as transparent in Imperialism BMPs
const TRANSPARENCY_COLOR: (u8, u8, u8) = (255, 0, 255);

#[derive(Default)]
pub struct ImperialismBmpLoader;

#[derive(Debug, Error)]
pub enum BmpLoaderError {
    #[error("Failed to read BMP file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to decode BMP: {0}")]
    ImageError(#[from] image::ImageError),
}

impl AssetLoader for ImperialismBmpLoader {
    type Asset = Image;
    type Settings = ();
    type Error = BmpLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        // Read the entire file into memory
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        info!("Loading BMP: {}", load_context.path().display());

        // Decode the BMP using the image crate
        let img = image::load_from_memory_with_format(&bytes, ImageFormat::Bmp)?;

        info!("BMP loaded: {}x{}", img.width(), img.height());

        // Convert to RGBA and handle transparency
        let rgba_img =
            convert_with_transparency(img, load_context.path().to_string_lossy().to_string());

        // Convert to Bevy Image
        let (width, height) = rgba_img.dimensions();
        let raw_data = rgba_img.into_raw();

        Ok(Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            raw_data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ))
    }

    fn extensions(&self) -> &[&str] {
        &["bmp"]
    }
}

/// Convert an image to RGBA, making the transparency color fully transparent
fn convert_with_transparency(
    img: DynamicImage,
    filename: String,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let rgb_img = img.to_rgb8();
    let (width, height) = rgb_img.dimensions();
    let mut rgba_img = ImageBuffer::new(width, height);

    let mut transparent_pixels = 0;
    let mut total_pixels = 0;
    let mut sample_logged = false;

    for (x, y, pixel) in rgb_img.enumerate_pixels() {
        let r = pixel[0];
        let g = pixel[1];
        let b = pixel[2];

        total_pixels += 1;

        // Log first few non-background pixels to see what colors we're getting
        if !sample_logged && (r != 0 || g != 0 || b != 0) && (r, g, b) != TRANSPARENCY_COLOR {
            info!("Sample pixel from {}: RGB({}, {}, {})", filename, r, g, b);
            sample_logged = true;
        }

        let rgba_pixel = if (r, g, b) == TRANSPARENCY_COLOR {
            // Make this pixel fully transparent
            transparent_pixels += 1;
            Rgba([0, 0, 0, 0]) // Fully transparent
        } else {
            // Keep this pixel fully opaque
            Rgba([r, g, b, 255])
        };

        rgba_img.put_pixel(x, y, rgba_pixel);
    }

    info!(
        "{}: Converted {} transparent pixels out of {} total ({:.1}%)",
        filename,
        transparent_pixels,
        total_pixels,
        (transparent_pixels as f32 / total_pixels as f32 * 100.0)
    );

    rgba_img
}

/// Plugin to register the Imperialism BMP loader
pub struct ImperialismBmpLoaderPlugin;

impl Plugin for ImperialismBmpLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_loader(ImperialismBmpLoader);
    }
}
