//! AVIF encode adapter using ravif.

use crate::config::CodecConfig;
use crate::pixel::{ImgRef, Rgb, Rgba};
use crate::{CodecError, EncodeOutput, ImageFormat, ImageMetadata, Limits, Stop};

/// Encode RGB8 pixels to AVIF.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let encoder = build_ravif_encoder(quality, metadata, codec_config);

    let result = encoder
        .encode_rgb(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?;

    Ok(EncodeOutput::new(result.avif_file, ImageFormat::Avif))
}

/// Encode RGBA8 pixels to AVIF.
pub(crate) fn encode_rgba8(
    img: ImgRef<Rgba<u8>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let encoder = build_ravif_encoder(quality, metadata, codec_config);

    let result = encoder
        .encode_rgba(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?;

    Ok(EncodeOutput::new(result.avif_file, ImageFormat::Avif))
}

/// Build a ravif Encoder from zencodecs parameters.
fn build_ravif_encoder<'a>(
    quality: Option<f32>,
    metadata: Option<&'a ImageMetadata<'a>>,
    codec_config: Option<&CodecConfig>,
) -> ravif::Encoder<'a> {
    let q = codec_config
        .and_then(|c| c.avif_quality)
        .or(quality)
        .unwrap_or(75.0)
        .clamp(0.0, 100.0);

    let speed = codec_config.and_then(|c| c.avif_speed).unwrap_or(4);

    let mut encoder = ravif::Encoder::new().with_quality(q).with_speed(speed);

    if let Some(alpha_q) = codec_config.and_then(|c| c.avif_alpha_quality) {
        encoder = encoder.with_alpha_quality(alpha_q);
    }

    // ravif supports EXIF only (no ICC, no XMP)
    if let Some(meta) = metadata {
        if let Some(exif) = meta.exif {
            encoder = encoder.with_exif(exif);
        }
    }

    encoder
}
