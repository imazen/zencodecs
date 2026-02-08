//! WebP codec adapter using zenwebp.

use crate::pixel::{ImgRef, ImgVec, Rgb, Rgba};
use crate::{CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, Limits, PixelData, Stop};

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

    Ok(ImageInfo {
        width,
        height,
        format: ImageFormat::WebP,
        has_alpha,
        has_animation,
        frame_count: Some(frame_count),
        icc_profile,
    })
}

/// Decode WebP to pixels.
pub(crate) fn decode(
    data: &[u8],
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    // Detect alpha to decide RGB vs RGBA decode
    let decoder = zenwebp::WebPDecoder::new(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
    let has_alpha = decoder.has_alpha();

    let demuxer = zenwebp::WebPDemuxer::new(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
    let has_animation = demuxer.is_animated();
    let icc_profile = demuxer.icc_profile().map(|p| p.to_vec());

    let pixels = if has_alpha {
        let (raw, width, height) = zenwebp::decode_rgba(data)
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
        let rgba_pixels: &[Rgba<u8>] = bytemuck::cast_slice(&raw);
        PixelData::Rgba8(ImgVec::new(rgba_pixels.to_vec(), width as usize, height as usize))
    } else {
        let (raw, width, height) = zenwebp::decode_rgb(data)
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
        let rgb_pixels: &[Rgb<u8>] = bytemuck::cast_slice(&raw);
        PixelData::Rgb8(ImgVec::new(rgb_pixels.to_vec(), width as usize, height as usize))
    };

    let width = pixels.width();
    let height = pixels.height();

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
        },
    })
}

/// Encode RGB8 pixels to WebP.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    quality: Option<f32>,
    lossless: bool,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let width = img.width() as u32;
    let height = img.height() as u32;
    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());

    let webp_data = if lossless {
        let config = zenwebp::LosslessConfig::new();
        zenwebp::EncodeRequest::lossless(&config, bytes, zenwebp::PixelLayout::Rgb8, width, height)
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    } else {
        let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0);
        let config = zenwebp::LossyConfig::new().with_quality(quality);
        zenwebp::EncodeRequest::lossy(&config, bytes, zenwebp::PixelLayout::Rgb8, width, height)
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    };

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
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let width = img.width() as u32;
    let height = img.height() as u32;
    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());

    let webp_data = if lossless {
        let config = zenwebp::LosslessConfig::new();
        zenwebp::EncodeRequest::lossless(&config, bytes, zenwebp::PixelLayout::Rgba8, width, height)
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    } else {
        let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0);
        let config = zenwebp::LossyConfig::new().with_quality(quality);
        zenwebp::EncodeRequest::lossy(&config, bytes, zenwebp::PixelLayout::Rgba8, width, height)
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    };

    Ok(EncodeOutput {
        data: webp_data,
        format: ImageFormat::WebP,
    })
}
