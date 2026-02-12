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

    Ok(DecodeOutput {
        pixels: result.pixels,
        info: convert_info(&result.info),
        #[cfg(feature = "jpeg")]
        jpeg_extras: None,
    })
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
    Ok(EncodeOutput {
        data,
        format: ImageFormat::Png,
    })
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
    Ok(EncodeOutput {
        data,
        format: ImageFormat::Png,
    })
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
    ImageInfo {
        width: info.width,
        height: info.height,
        format: ImageFormat::Png,
        has_alpha: info.has_alpha,
        has_animation: info.has_animation,
        frame_count: Some(info.frame_count),
        icc_profile: info.icc_profile.clone(),
        exif: info.exif.clone(),
        xmp: info.xmp.clone(),
    }
}
