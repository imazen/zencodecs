//! JXL encode adapter — delegates to zenjxl.

use crate::config::CodecConfig;
use crate::pixel::{Bgra, ImgRef, Rgb, Rgba};
use crate::{CodecError, EncodeOutput, ImageFormat, ImageMetadata, Limits, Stop};

/// Encode RGB8 pixels to JXL.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    quality: Option<f32>,
    _metadata: Option<&ImageMetadata<'_>>,
    _codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let config = build_lossy_config(quality);
    let data = zenjxl::encode_rgb8(img, &config)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))?;
    Ok(EncodeOutput {
        data,
        format: ImageFormat::Jxl,
    })
}

/// Encode RGBA8 pixels to JXL.
pub(crate) fn encode_rgba8(
    img: ImgRef<Rgba<u8>>,
    quality: Option<f32>,
    _metadata: Option<&ImageMetadata<'_>>,
    _codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let config = build_lossy_config(quality);
    let data = zenjxl::encode_rgba8(img, &config)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))?;
    Ok(EncodeOutput {
        data,
        format: ImageFormat::Jxl,
    })
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
    let config = build_lossy_config(quality);
    let data = zenjxl::encode_bgra8(img, &config)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))?;
    Ok(EncodeOutput {
        data,
        format: ImageFormat::Jxl,
    })
}

fn build_lossy_config(quality: Option<f32>) -> zenjxl::LossyConfig {
    let distance = quality.map_or(1.0, percent_to_distance);
    zenjxl::LossyConfig::new(distance)
}

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
