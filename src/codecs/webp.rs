//! WebP codec adapter using zenwebp.

use crate::config::CodecConfig;
use crate::pixel::{Bgra, ImgRef, ImgVec, Rgb, Rgba};
use crate::{
    CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, ImageMetadata, Limits,
    PixelData, Stop,
};

/// Probe WebP metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let info = zenwebp::ImageInfo::from_webp(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;

    Ok(ImageInfo {
        width: info.width,
        height: info.height,
        format: ImageFormat::WebP,
        has_alpha: info.has_alpha,
        has_animation: info.has_animation,
        frame_count: Some(info.frame_count),
        icc_profile: info.icc_profile,
        exif: info.exif,
        xmp: info.xmp,
    })
}

/// Build zenwebp DecodeConfig from codec config and limits.
fn build_decode_config(
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
) -> zenwebp::DecodeConfig {
    // Start from the user's config if provided, otherwise default.
    let mut config = codec_config
        .and_then(|c| c.webp_decoder.as_ref())
        .map(|c| c.as_ref().clone())
        .unwrap_or_default();

    // Merge zencodecs limits into the config's limits.
    if let Some(lim) = limits {
        let mut wl = config.limits.clone();
        if let Some(max_w) = lim.max_width {
            wl.max_width = Some(max_w as u32);
        }
        if let Some(max_h) = lim.max_height {
            wl.max_height = Some(max_h as u32);
        }
        if let Some(max_px) = lim.max_pixels {
            wl.max_total_pixels = Some(max_px);
        }
        if let Some(max_mem) = lim.max_memory_bytes {
            wl.max_memory = Some(max_mem);
        }
        config = config.limits(wl);
    }

    config
}

/// Decode WebP to pixels.
pub(crate) fn decode(
    data: &[u8],
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    // Get metadata via lightweight probe (parses headers, no pixel decode).
    let webp_info = zenwebp::ImageInfo::from_webp(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;

    let decode_config = build_decode_config(codec_config, limits);

    let mut request = zenwebp::DecodeRequest::new(&decode_config, data);
    if let Some(s) = stop {
        request = request.stop(s);
    }

    // decode() auto-selects RGB or RGBA based on image content.
    let (raw, width, height, layout) = request
        .decode()
        .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;

    let has_alpha = layout == zenwebp::PixelLayout::Rgba8;
    let pixels = if has_alpha {
        let rgba_pixels: &[Rgba<u8>] = bytemuck::cast_slice(&raw);
        PixelData::Rgba8(ImgVec::new(
            rgba_pixels.to_vec(),
            width as usize,
            height as usize,
        ))
    } else {
        let rgb_pixels: &[Rgb<u8>] = bytemuck::cast_slice(&raw);
        PixelData::Rgb8(ImgVec::new(
            rgb_pixels.to_vec(),
            width as usize,
            height as usize,
        ))
    };

    // Additional zencodecs-level limits check
    if let Some(lim) = limits {
        let bpp = if has_alpha { 4 } else { 3 };
        lim.validate(width, height, bpp)?;
    }

    Ok(DecodeOutput {
        pixels,
        info: ImageInfo {
            width,
            height,
            format: ImageFormat::WebP,
            has_alpha: webp_info.has_alpha,
            has_animation: webp_info.has_animation,
            frame_count: Some(webp_info.frame_count),
            icc_profile: webp_info.icc_profile,
            exif: webp_info.exif,
            xmp: webp_info.xmp,
        },
        #[cfg(feature = "jpeg")]
        jpeg_extras: None,
    })
}

/// Convert zencodecs ImageMetadata to zenwebp ImageMetadata.
fn to_webp_metadata<'a>(metadata: Option<&'a ImageMetadata<'a>>) -> zenwebp::ImageMetadata<'a> {
    let mut webp_meta = zenwebp::ImageMetadata::new();
    if let Some(meta) = metadata {
        if let Some(icc) = meta.icc_profile {
            webp_meta = webp_meta.with_icc_profile(icc);
        }
        if let Some(exif) = meta.exif {
            webp_meta = webp_meta.with_exif(exif);
        }
        if let Some(xmp) = meta.xmp {
            webp_meta = webp_meta.with_xmp(xmp);
        }
    }
    webp_meta
}

/// Encode RGB8 pixels to WebP.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    quality: Option<f32>,
    lossless: bool,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let width = img.width() as u32;
    let height = img.height() as u32;
    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());
    let webp_meta = to_webp_metadata(metadata);

    let webp_data = if lossless {
        let config = codec_config
            .and_then(|c| c.webp_lossless.as_ref())
            .map(|c| c.as_ref().clone())
            .unwrap_or_default();
        let mut request = zenwebp::EncodeRequest::lossless(
            &config,
            bytes,
            zenwebp::PixelLayout::Rgb8,
            width,
            height,
        )
        .with_metadata(webp_meta);
        if let Some(s) = stop {
            request = request.with_stop(s);
        }
        request
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    } else {
        let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0);
        let config = codec_config
            .and_then(|c| c.webp_lossy.as_ref())
            .map(|c| c.as_ref().clone())
            .unwrap_or_else(|| zenwebp::LossyConfig::new().with_quality(quality));
        let mut request = zenwebp::EncodeRequest::lossy(
            &config,
            bytes,
            zenwebp::PixelLayout::Rgb8,
            width,
            height,
        )
        .with_metadata(webp_meta);
        if let Some(s) = stop {
            request = request.with_stop(s);
        }
        request
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    };

    Ok(EncodeOutput::new(webp_data, ImageFormat::WebP))
}

/// Encode RGBA8 pixels to WebP.
pub(crate) fn encode_rgba8(
    img: ImgRef<Rgba<u8>>,
    quality: Option<f32>,
    lossless: bool,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let width = img.width() as u32;
    let height = img.height() as u32;
    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());
    let webp_meta = to_webp_metadata(metadata);

    let webp_data = if lossless {
        let config = codec_config
            .and_then(|c| c.webp_lossless.as_ref())
            .map(|c| c.as_ref().clone())
            .unwrap_or_default();
        let mut request = zenwebp::EncodeRequest::lossless(
            &config,
            bytes,
            zenwebp::PixelLayout::Rgba8,
            width,
            height,
        )
        .with_metadata(webp_meta);
        if let Some(s) = stop {
            request = request.with_stop(s);
        }
        request
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    } else {
        let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0);
        let config = codec_config
            .and_then(|c| c.webp_lossy.as_ref())
            .map(|c| c.as_ref().clone())
            .unwrap_or_else(|| zenwebp::LossyConfig::new().with_quality(quality));
        let mut request = zenwebp::EncodeRequest::lossy(
            &config,
            bytes,
            zenwebp::PixelLayout::Rgba8,
            width,
            height,
        )
        .with_metadata(webp_meta);
        if let Some(s) = stop {
            request = request.with_stop(s);
        }
        request
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    };

    Ok(EncodeOutput::new(webp_data, ImageFormat::WebP))
}

/// Encode BGRA8 pixels to WebP (native BGRA path).
pub(crate) fn encode_bgra8(
    img: ImgRef<Bgra<u8>>,
    quality: Option<f32>,
    lossless: bool,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let width = img.width() as u32;
    let height = img.height() as u32;
    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());
    let webp_meta = to_webp_metadata(metadata);

    let webp_data = if lossless {
        let config = codec_config
            .and_then(|c| c.webp_lossless.as_ref())
            .map(|c| c.as_ref().clone())
            .unwrap_or_default();
        let mut request = zenwebp::EncodeRequest::lossless(
            &config,
            bytes,
            zenwebp::PixelLayout::Bgra8,
            width,
            height,
        )
        .with_metadata(webp_meta);
        if let Some(s) = stop {
            request = request.with_stop(s);
        }
        request
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    } else {
        let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0);
        let config = codec_config
            .and_then(|c| c.webp_lossy.as_ref())
            .map(|c| c.as_ref().clone())
            .unwrap_or_else(|| zenwebp::LossyConfig::new().with_quality(quality));
        let mut request = zenwebp::EncodeRequest::lossy(
            &config,
            bytes,
            zenwebp::PixelLayout::Bgra8,
            width,
            height,
        )
        .with_metadata(webp_meta);
        if let Some(s) = stop {
            request = request.with_stop(s);
        }
        request
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    };

    Ok(EncodeOutput::new(webp_data, ImageFormat::WebP))
}
