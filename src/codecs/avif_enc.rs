//! AVIF encode adapter using ravif.

use crate::{CodecError, EncodeOutput, ImageFormat, Limits, PixelLayout, Stop};

/// Encode pixels to AVIF.
#[allow(clippy::too_many_arguments)]
pub(crate) fn encode(
    pixels: &[u8],
    width: u32,
    height: u32,
    layout: PixelLayout,
    quality: Option<f32>,
    _lossless: bool, // AVIF doesn't have a simple lossless flag (quality 100 is near-lossless)
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    // ravif quality is 0-100, with 100 being near-lossless
    let quality = quality.unwrap_or(75.0).clamp(0.0, 100.0);

    // Create encoder with quality settings
    let encoder = ravif::Encoder::new()
        .with_quality(quality)
        .with_speed(4); // Default speed (0=slowest/best, 10=fastest/worst)

    // Convert pixels to rgb crate types and encode
    let avif_data = match layout {
        PixelLayout::Rgba8 => {
            // Convert &[u8] to &[RGBA<u8>] using bytemuck
            let rgba_pixels: &[rgb::RGBA<u8>] = rgb::bytemuck::cast_slice(pixels);

            // Create Img wrapper for ravif
            let img = imgref::Img::new(rgba_pixels, width as usize, height as usize);

            // Encode
            encoder
                .encode_rgba(img)
                .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?
                .avif_file
        }
        PixelLayout::Rgb8 => {
            // Convert &[u8] to &[RGB<u8>] using bytemuck
            let rgb_pixels: &[rgb::RGB<u8>] = rgb::bytemuck::cast_slice(pixels);

            // Create Img wrapper for ravif
            let img = imgref::Img::new(rgb_pixels, width as usize, height as usize);

            // Encode
            encoder
                .encode_rgb(img)
                .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?
                .avif_file
        }
        PixelLayout::Bgra8 => {
            // Convert BGRA to RGBA first
            let mut rgba = alloc::vec::Vec::with_capacity(pixels.len());
            for chunk in pixels.chunks_exact(4) {
                rgba.push(chunk[2]); // R
                rgba.push(chunk[1]); // G
                rgba.push(chunk[0]); // B
                rgba.push(chunk[3]); // A
            }

            // Convert to &[RGBA<u8>] using bytemuck
            let rgba_pixels: &[rgb::RGBA<u8>] = rgb::bytemuck::cast_slice(&rgba);

            // Create Img wrapper and encode
            let img = imgref::Img::new(rgba_pixels, width as usize, height as usize);
            encoder
                .encode_rgba(img)
                .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?
                .avif_file
        }
        PixelLayout::Bgr8 => {
            // Convert BGR to RGB first
            let mut rgb = alloc::vec::Vec::with_capacity(pixels.len());
            for chunk in pixels.chunks_exact(3) {
                rgb.push(chunk[2]); // R
                rgb.push(chunk[1]); // G
                rgb.push(chunk[0]); // B
            }

            // Convert to &[RGB<u8>] using bytemuck
            let rgb_pixels: &[rgb::RGB<u8>] = rgb::bytemuck::cast_slice(&rgb);

            // Create Img wrapper and encode
            let img = imgref::Img::new(rgb_pixels, width as usize, height as usize);
            encoder
                .encode_rgb(img)
                .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?
                .avif_file
        }
    };

    Ok(EncodeOutput {
        data: avif_data,
        format: ImageFormat::Avif,
    })
}
