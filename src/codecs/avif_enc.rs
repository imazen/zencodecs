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

    // Swap BGR→RGB if needed (ravif only supports RGB-order)
    let (rgb_pixels, rgb_layout) = crate::pixel::to_rgb_order(pixels, layout);

    // Encode based on whether we have alpha
    let avif_data = if rgb_layout.has_alpha() {
        let rgba_pixels: &[rgb::RGBA<u8>] = rgb::bytemuck::cast_slice(&rgb_pixels);
        let img = imgref::Img::new(rgba_pixels, width as usize, height as usize);
        encoder
            .encode_rgba(img)
            .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?
            .avif_file
    } else {
        let rgb_pixels: &[rgb::RGB<u8>] = rgb::bytemuck::cast_slice(&rgb_pixels);
        let img = imgref::Img::new(rgb_pixels, width as usize, height as usize);
        encoder
            .encode_rgb(img)
            .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?
            .avif_file
    };

    Ok(EncodeOutput {
        data: avif_data,
        format: ImageFormat::Avif,
    })
}
