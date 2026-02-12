//! PNG codec adapter — delegates to zenpng.

use crate::config::CodecConfig;
use crate::pixel::{ImgRef, Rgb, Rgba};
use crate::{
    CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, ImageMetadata, Limits, Stop,
};

/// Probe PNG metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let info = zenpng::probe(data).map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;
    Ok(convert_info(&info))
}

/// Decode PNG to pixels.
pub(crate) fn decode(
    data: &[u8],
    limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let png_limits = limits.map(|lim| zenpng::PngLimits {
        max_pixels: lim.max_pixels,
        max_memory_bytes: lim.max_memory_bytes,
    });

    let result = zenpng::decode(data, png_limits.as_ref())
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;

    Ok(DecodeOutput::new(result.pixels, convert_info(&result.info)))
}

/// Encode RGB8 pixels to PNG.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let config = build_encode_config(codec_config);
    let data = zenpng::encode_rgb8(img, metadata, &config)
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;
    Ok(EncodeOutput::new(data, ImageFormat::Png))
}

/// Encode RGBA8 pixels to PNG.
pub(crate) fn encode_rgba8(
    img: ImgRef<Rgba<u8>>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let config = build_encode_config(codec_config);
    let data = zenpng::encode_rgba8(img, metadata, &config)
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;
    Ok(EncodeOutput::new(data, ImageFormat::Png))
}

fn build_encode_config(codec_config: Option<&CodecConfig>) -> zenpng::EncodeConfig {
    let mut config = zenpng::EncodeConfig::default();
    if let Some(cfg) = codec_config {
        if let Some(compression) = cfg.png_compression {
            config.compression = compression;
        }
        if let Some(filter) = cfg.png_filter {
            config.filter = filter;
        }
    }
    config
}

fn convert_info(info: &zenpng::PngInfo) -> ImageInfo {
    let mut ii = ImageInfo::new(info.width, info.height, ImageFormat::Png)
        .with_alpha(info.has_alpha)
        .with_animation(info.has_animation)
        .with_frame_count(info.frame_count);
    if let Some(ref icc) = info.icc_profile {
        ii = ii.with_icc_profile(icc.clone());
    }
    if let Some(ref exif) = info.exif {
        ii = ii.with_exif(exif.clone());
    }
    if let Some(ref xmp) = info.xmp {
        ii = ii.with_xmp(xmp.clone());
    }
    ii
}
