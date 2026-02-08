//! AVIF encode adapter using ravif.

use crate::pixel::{ImgRef, Rgb, Rgba};
use crate::{CodecError, EncodeOutput, ImageFormat, Limits, Stop};

/// Encode RGB8 pixels to AVIF.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    quality: Option<f32>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let quality = quality.unwrap_or(75.0).clamp(0.0, 100.0);

    let encoder = ravif::Encoder::new()
        .with_quality(quality)
        .with_speed(4);

    let result = encoder
        .encode_rgb(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?;

    Ok(EncodeOutput {
        data: result.avif_file,
        format: ImageFormat::Avif,
    })
}

/// Encode RGBA8 pixels to AVIF.
pub(crate) fn encode_rgba8(
    img: ImgRef<Rgba<u8>>,
    quality: Option<f32>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let quality = quality.unwrap_or(75.0).clamp(0.0, 100.0);

    let encoder = ravif::Encoder::new()
        .with_quality(quality)
        .with_speed(4);

    let result = encoder
        .encode_rgba(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?;

    Ok(EncodeOutput {
        data: result.avif_file,
        format: ImageFormat::Avif,
    })
}
