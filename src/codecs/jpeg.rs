//! JPEG codec adapter using zenjpeg.

use crate::{
    CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, Limits, PixelLayout, Stop,
};

/// Convert zencodecs PixelLayout to zenjpeg encoder PixelLayout.
fn to_jpeg_encoder_layout(layout: PixelLayout) -> zenjpeg::encoder::PixelLayout {
    match layout {
        PixelLayout::Rgb8 => zenjpeg::encoder::PixelLayout::Rgb8Srgb,
        PixelLayout::Rgba8 => zenjpeg::encoder::PixelLayout::Rgba8Srgb,
        PixelLayout::Bgr8 => zenjpeg::encoder::PixelLayout::Bgr8Srgb,
        PixelLayout::Bgra8 => zenjpeg::encoder::PixelLayout::Bgra8Srgb,
    }
}

// TODO: Add proper limits conversion once API is understood better

/// Probe JPEG metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    // Use zenjpeg's Decoder to get image info
    let decoder = zenjpeg::decoder::Decoder::new();
    let result = decoder
        .decode(data, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    Ok(ImageInfo {
        width: result.width(),
        height: result.height(),
        format: ImageFormat::Jpeg,
        has_alpha: false,     // JPEG doesn't support alpha
        has_animation: false, // JPEG doesn't support animation
        frame_count: Some(1),
        icc_profile: None, // TODO: extract ICC profile from decoder
    })
}

/// Decode JPEG to pixels.
pub(crate) fn decode(
    data: &[u8],
    _output_layout: PixelLayout,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    // Create decoder
    let decoder = zenjpeg::decoder::Decoder::new();

    // Decode (TODO: use stop token when API supports it)
    let decoded = decoder
        .decode(data, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    // Extract pixel data - zenjpeg decoder returns RGB8 by default
    let pixels = decoded
        .pixels_u8()
        .ok_or_else(|| CodecError::InvalidInput("no pixel data in decoded image".into()))?
        .to_vec();

    // Get format from decoder (RGB or Grayscale)
    let layout = match decoded.format() {
        zenjpeg::decoder::PixelFormat::Rgb => PixelLayout::Rgb8,
        zenjpeg::decoder::PixelFormat::Gray => PixelLayout::Rgb8, // Gray expanded to RGB
        _ => PixelLayout::Rgb8,                                   // Default to RGB8
    };

    Ok(DecodeOutput {
        pixels,
        width: decoded.width(),
        height: decoded.height(),
        layout,
        info: ImageInfo {
            width: decoded.width(),
            height: decoded.height(),
            format: ImageFormat::Jpeg,
            has_alpha: false,
            has_animation: false,
            frame_count: Some(1),
            icc_profile: None, // TODO: extract ICC profile
        },
    })
}

/// Encode pixels to JPEG.
#[allow(clippy::too_many_arguments)]
pub(crate) fn encode(
    pixels: &[u8],
    width: u32,
    height: u32,
    layout: PixelLayout,
    quality: Option<f32>,
    _lossless: bool, // JPEG doesn't support lossless
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    // JPEG quality is 0-100 (native scale)
    let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0) as u8;

    // Create encoder config (YCbCr mode, 4:2:0 chroma subsampling)
    let config = zenjpeg::encoder::EncoderConfig::ycbcr(
        quality,
        zenjpeg::encoder::ChromaSubsampling::Quarter, // 4:2:0
    );

    // Create encoder
    let pixel_layout = to_jpeg_encoder_layout(layout);
    let mut encoder = config
        .encode_from_bytes(width, height, pixel_layout)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    // Push pixels (TODO: use stop token when API supports it)
    encoder
        .push_packed(pixels, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    // Finish encoding
    let jpeg_data = encoder
        .finish()
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    Ok(EncodeOutput {
        data: jpeg_data,
        format: ImageFormat::Jpeg,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_format_conversion() {
        // Just verify all formats can be converted
        for layout in [
            PixelLayout::Rgb8,
            PixelLayout::Rgba8,
            PixelLayout::Bgr8,
            PixelLayout::Bgra8,
        ] {
            let _jpeg_format = to_jpeg_encoder_layout(layout);
        }
    }
}
