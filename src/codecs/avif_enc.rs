//! AVIF encode adapter using zenavif via trait interface.

use crate::config::CodecConfig;
use crate::limits::to_resource_limits;
use crate::pixel::{ImgRef, Rgb, Rgba};
use crate::{
    CodecError, EncodeOutput, Encoding, EncodingJob, ImageFormat, ImageMetadata, Limits, Stop,
};

/// Encode RGB8 pixels to AVIF.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, codec_config, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_rgb8(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))
}

/// Encode RGBA8 pixels to AVIF.
pub(crate) fn encode_rgba8(
    img: ImgRef<Rgba<u8>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, codec_config, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_rgba8(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))
}

/// Encode Gray8 pixels to AVIF.
pub(crate) fn encode_gray8(
    img: ImgRef<crate::pixel::Gray<u8>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, codec_config, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_gray8(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))
}

/// Encode linear RGB f32 pixels to AVIF.
pub(crate) fn encode_rgb_f32(
    img: ImgRef<Rgb<f32>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, codec_config, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_rgb_f32(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))
}

/// Encode linear RGBA f32 pixels to AVIF.
pub(crate) fn encode_rgba_f32(
    img: ImgRef<Rgba<f32>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, codec_config, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_rgba_f32(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))
}

/// Encode linear grayscale f32 pixels to AVIF.
pub(crate) fn encode_gray_f32(
    img: ImgRef<crate::pixel::Gray<f32>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, codec_config, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_gray_f32(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))
}

/// Build an AvifEncoding from quality/codec_config/limits.
fn build_encoding(
    quality: Option<f32>,
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
) -> zenavif::AvifEncoding {
    let q = codec_config
        .and_then(|c| c.avif_quality)
        .or(quality)
        .unwrap_or(75.0)
        .clamp(0.0, 100.0);

    let speed = codec_config.and_then(|c| c.avif_speed).unwrap_or(4);

    let mut enc = zenavif::AvifEncoding::new()
        .with_quality(q)
        .with_effort(speed as u32);

    if let Some(alpha_q) = codec_config.and_then(|c| c.avif_alpha_quality) {
        enc = enc.with_alpha_quality(alpha_q);
    }

    if let Some(lim) = limits {
        enc = enc.with_limits(to_resource_limits(lim));
    }
    enc
}
