//! JXL encode adapter — delegates to zenjxl via trait interface.

use crate::config::CodecConfig;
use crate::limits::to_resource_limits;
use crate::pixel::{Bgra, ImgRef, Rgb, Rgba};
use crate::{
    CodecError, EncodeOutput, Encoding, EncodingJob, ImageFormat, ImageMetadata, Limits, Stop,
};

/// Map 0-100 quality percentage to butteraugli distance.
fn percent_to_distance(quality: f32) -> f32 {
    let q = quality.clamp(0.0, 99.9) as u32;
    if q >= 90 {
        (100 - q) as f32 / 10.0
    } else if q >= 70 {
        1.0 + (90 - q) as f32 / 20.0
    } else {
        2.0 + (70 - q) as f32 / 10.0
    }
}

/// Build a JxlEncoding from quality and limits.
fn build_encoding(quality: Option<f32>, limits: Option<&Limits>) -> zenjxl::JxlEncoding {
    let distance = quality.map_or(1.0, percent_to_distance);
    let mut enc = zenjxl::JxlEncoding::lossy(distance);
    if let Some(lim) = limits {
        enc = enc.with_limits(to_resource_limits(lim));
    }
    enc
}

/// Encode RGB8 pixels to JXL.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    _codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_rgb8(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))
}

/// Encode RGBA8 pixels to JXL.
pub(crate) fn encode_rgba8(
    img: ImgRef<Rgba<u8>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    _codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_rgba8(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))
}

/// Encode Gray8 pixels to JXL.
pub(crate) fn encode_gray8(
    img: ImgRef<crate::pixel::Gray<u8>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    _codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_gray8(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))
}

/// Encode linear RGB f32 pixels to JXL.
pub(crate) fn encode_rgb_f32(
    img: ImgRef<Rgb<f32>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    _codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_rgb_f32(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))
}

/// Encode linear RGBA f32 pixels to JXL.
pub(crate) fn encode_rgba_f32(
    img: ImgRef<Rgba<f32>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    _codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_rgba_f32(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))
}

/// Encode linear grayscale f32 pixels to JXL.
pub(crate) fn encode_gray_f32(
    img: ImgRef<crate::pixel::Gray<f32>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    _codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_gray_f32(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))
}

/// Encode BGRA8 pixels to JXL (native BGRA path).
pub(crate) fn encode_bgra8(
    img: ImgRef<Bgra<u8>>,
    quality: Option<f32>,
    _metadata: Option<&ImageMetadata<'_>>,
    _codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let distance = quality.map_or(1.0, percent_to_distance);
    let config = zenjxl::LossyConfig::new(distance);
    let data = zenjxl::encode_bgra8(img, &config)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))?;
    Ok(EncodeOutput::new(data, ImageFormat::Jxl))
}
