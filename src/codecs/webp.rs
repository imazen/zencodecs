//! WebP codec adapter using zenwebp.

use crate::config::CodecConfig;
use crate::pixel::{ImgRef, ImgVec, Rgb, Rgba};
use crate::{
    CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, ImageMetadata, Limits,
    PixelData, Stop,
};

/// Probe WebP metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let decoder = zenwebp::WebPDecoder::new(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;

    let (width, height) = decoder.dimensions();
    let has_alpha = decoder.has_alpha();

    let demuxer = zenwebp::WebPDemuxer::new(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
    let has_animation = demuxer.is_animated();
    let frame_count = demuxer.num_frames();
    let icc_profile = demuxer.icc_profile().map(|p| p.to_vec());
    let exif = demuxer.exif().map(|p| p.to_vec());
    let xmp = demuxer.xmp().map(|p| p.to_vec());

    Ok(ImageInfo {
        width,
        height,
        format: ImageFormat::WebP,
        has_alpha,
        has_animation,
        frame_count: Some(frame_count),
        icc_profile,
        exif,
        xmp,
    })
}

/// Build zenwebp Limits from zencodecs Limits.
fn to_webp_limits(limits: Option<&Limits>) -> zenwebp::decoder::Limits {
    let mut wl = zenwebp::decoder::Limits::default();
    if let Some(lim) = limits {
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
    }
    wl
}

/// Decode WebP to pixels.
pub(crate) fn decode(
    data: &[u8],
    _codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let demuxer = zenwebp::WebPDemuxer::new(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
    let has_animation = demuxer.is_animated();
    let icc_profile = demuxer.icc_profile().map(|p| p.to_vec());
    let exif = demuxer.exif().map(|p| p.to_vec());
    let xmp = demuxer.xmp().map(|p| p.to_vec());

    let webp_config = zenwebp::DecodeConfig::default().limits(to_webp_limits(limits));

    let mut request = zenwebp::DecodeRequest::new(&webp_config, data);
    if let Some(s) = stop {
        request = request.stop(s);
    }

    let has_alpha = zenwebp::WebPDecoder::new(data)
        .map(|d| d.has_alpha())
        .unwrap_or(false);

    let pixels = if has_alpha {
        let (raw, width, height) = request
            .decode_rgba()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
        let rgba_pixels: &[Rgba<u8>] = bytemuck::cast_slice(&raw);
        PixelData::Rgba8(ImgVec::new(
            rgba_pixels.to_vec(),
            width as usize,
            height as usize,
        ))
    } else {
        let (raw, width, height) = request
            .decode_rgb()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
        let rgb_pixels: &[Rgb<u8>] = bytemuck::cast_slice(&raw);
        PixelData::Rgb8(ImgVec::new(
            rgb_pixels.to_vec(),
            width as usize,
            height as usize,
        ))
    };

    let width = pixels.width();
    let height = pixels.height();

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
            has_alpha,
            has_animation,
            frame_count: Some(demuxer.num_frames()),
            icc_profile,
            exif,
            xmp,
        },
        #[cfg(feature = "jpeg")]
        jpeg_extras: None,
    })
}

/// Encode RGB8 pixels to WebP.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    quality: Option<f32>,
    lossless: bool,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let width = img.width() as u32;
    let height = img.height() as u32;
    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());

    let mut webp_data = if lossless {
        let config = codec_config
            .and_then(|c| c.webp_lossless.as_ref())
            .map(|c| c.as_ref().clone())
            .unwrap_or_default();
        zenwebp::EncodeRequest::lossless(&config, bytes, zenwebp::PixelLayout::Rgb8, width, height)
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    } else {
        let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0);
        let config = codec_config
            .and_then(|c| c.webp_lossy.as_ref())
            .map(|c| c.as_ref().clone())
            .unwrap_or_else(|| zenwebp::LossyConfig::new().with_quality(quality));
        zenwebp::EncodeRequest::lossy(&config, bytes, zenwebp::PixelLayout::Rgb8, width, height)
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    };

    embed_webp_metadata(&mut webp_data, metadata)?;

    Ok(EncodeOutput {
        data: webp_data,
        format: ImageFormat::WebP,
    })
}

/// Encode RGBA8 pixels to WebP.
pub(crate) fn encode_rgba8(
    img: ImgRef<Rgba<u8>>,
    quality: Option<f32>,
    lossless: bool,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let width = img.width() as u32;
    let height = img.height() as u32;
    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());

    let mut webp_data = if lossless {
        let config = codec_config
            .and_then(|c| c.webp_lossless.as_ref())
            .map(|c| c.as_ref().clone())
            .unwrap_or_default();
        zenwebp::EncodeRequest::lossless(&config, bytes, zenwebp::PixelLayout::Rgba8, width, height)
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    } else {
        let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0);
        let config = codec_config
            .and_then(|c| c.webp_lossy.as_ref())
            .map(|c| c.as_ref().clone())
            .unwrap_or_else(|| zenwebp::LossyConfig::new().with_quality(quality));
        zenwebp::EncodeRequest::lossy(&config, bytes, zenwebp::PixelLayout::Rgba8, width, height)
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    };

    embed_webp_metadata(&mut webp_data, metadata)?;

    Ok(EncodeOutput {
        data: webp_data,
        format: ImageFormat::WebP,
    })
}

/// Embed ICC/EXIF/XMP metadata into encoded WebP data via mux.
fn embed_webp_metadata(
    webp_data: &mut alloc::vec::Vec<u8>,
    metadata: Option<&ImageMetadata<'_>>,
) -> Result<(), CodecError> {
    if let Some(meta) = metadata {
        if let Some(icc) = meta.icc_profile {
            *webp_data = zenwebp::embed_icc(webp_data, icc)
                .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
        }
        if let Some(exif) = meta.exif {
            *webp_data = zenwebp::embed_exif(webp_data, exif)
                .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
        }
        if let Some(xmp) = meta.xmp {
            *webp_data = zenwebp::embed_xmp(webp_data, xmp)
                .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
        }
    }
    Ok(())
}
