//! GIF codec adapter using zengif.

extern crate std;

use crate::config::CodecConfig;
use crate::pixel::{ImgRef, ImgVec, Rgb, Rgba};
use crate::{
    CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, Limits, PixelData, Stop,
};

/// Probe GIF metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let cursor = std::io::Cursor::new(data);
    let decoder = zengif::Decoder::new(cursor, zengif::Limits::default(), &enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Gif, e))?;

    let metadata = decoder.metadata();

    Ok(ImageInfo {
        width: metadata.width as u32,
        height: metadata.height as u32,
        format: ImageFormat::Gif,
        has_alpha: true,
        has_animation: true, // Can't know without parsing all frames; assume animated
        frame_count: None,   // Requires full decode to determine
        icc_profile: None,
        exif: None,
        xmp: None,
    })
}

/// Convert zencodecs Limits to zengif Limits.
fn to_gif_limits(limits: Option<&Limits>) -> zengif::Limits {
    let mut gif_limits = zengif::Limits::default();
    if let Some(lim) = limits {
        if let Some(max_w) = lim.max_width {
            gif_limits.max_width = Some(max_w.min(u16::MAX as u64) as u16);
        }
        if let Some(max_h) = lim.max_height {
            gif_limits.max_height = Some(max_h.min(u16::MAX as u64) as u16);
        }
        if let Some(max_px) = lim.max_pixels {
            gif_limits.max_total_pixels = Some(max_px);
        }
        if let Some(max_mem) = lim.max_memory_bytes {
            gif_limits.max_memory = Some(max_mem);
        }
    }
    gif_limits
}

/// Decode GIF to pixels (first frame only).
pub(crate) fn decode(
    data: &[u8],
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let stop = crate::limits::stop_or_default(stop);
    let gif_limits = to_gif_limits(limits);

    let cursor = std::io::Cursor::new(data);
    let mut decoder = zengif::Decoder::new(cursor, gif_limits, stop)
        .map_err(|e| CodecError::from_codec(ImageFormat::Gif, e))?;

    let metadata = decoder.metadata().clone();

    let frame = decoder
        .next_frame()
        .map_err(|e| CodecError::from_codec(ImageFormat::Gif, e))?
        .ok_or_else(|| CodecError::InvalidInput("GIF has no frames".into()))?;

    let width = metadata.width as usize;
    let height = metadata.height as usize;

    let rgba_pixels: alloc::vec::Vec<Rgba<u8>> = frame
        .pixels
        .into_iter()
        .map(Rgba::from)
        .collect();

    let img = ImgVec::new(rgba_pixels, width, height);

    // Count remaining frames to determine animation status
    let mut frame_count: u32 = 1;
    while decoder
        .next_frame()
        .map_err(|e| CodecError::from_codec(ImageFormat::Gif, e))?
        .is_some()
    {
        frame_count += 1;
    }

    Ok(DecodeOutput {
        pixels: PixelData::Rgba8(img),
        info: ImageInfo {
            width: width as u32,
            height: height as u32,
            format: ImageFormat::Gif,
            has_alpha: true,
            has_animation: frame_count > 1,
            frame_count: Some(frame_count),
            icc_profile: None,
            exif: None,
            xmp: None,
        },
        #[cfg(feature = "jpeg")]
        jpeg_extras: None,
    })
}

/// Encode RGB8 pixels to GIF (single frame).
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let width: u16 = (img.width() as u32)
        .try_into()
        .map_err(|_| CodecError::InvalidInput("width exceeds GIF maximum (65535)".into()))?;
    let height: u16 = (img.height() as u32)
        .try_into()
        .map_err(|_| CodecError::InvalidInput("height exceeds GIF maximum (65535)".into()))?;

    let (buf, _, _) = img.to_contiguous_buf();
    let rgba_bytes: alloc::vec::Vec<u8> = buf.iter().flat_map(|p| [p.r, p.g, p.b, 255u8]).collect();

    let frame = zengif::FrameInput::from_bytes(width, height, 10, &rgba_bytes);

    let config = codec_config
        .and_then(|c| c.gif_encoder.as_ref())
        .map(|c| c.as_ref().clone())
        .unwrap_or_else(zengif::EncoderConfig::new);
    let limits = zengif::Limits::default();

    let gif_data = zengif::encode_gif(
        alloc::vec![frame],
        width,
        height,
        config,
        limits,
        &enough::Unstoppable,
    )
    .map_err(|e| CodecError::from_codec(ImageFormat::Gif, e))?;

    Ok(EncodeOutput {
        data: gif_data,
        format: ImageFormat::Gif,
    })
}

/// Encode RGBA8 pixels to GIF (single frame).
pub(crate) fn encode_rgba8(
    img: ImgRef<Rgba<u8>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let width: u16 = (img.width() as u32)
        .try_into()
        .map_err(|_| CodecError::InvalidInput("width exceeds GIF maximum (65535)".into()))?;
    let height: u16 = (img.height() as u32)
        .try_into()
        .map_err(|_| CodecError::InvalidInput("height exceeds GIF maximum (65535)".into()))?;

    let (buf, _, _) = img.to_contiguous_buf();
    let rgba_bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());

    let frame = zengif::FrameInput::from_bytes(width, height, 10, rgba_bytes);

    let config = codec_config
        .and_then(|c| c.gif_encoder.as_ref())
        .map(|c| c.as_ref().clone())
        .unwrap_or_else(zengif::EncoderConfig::new);
    let limits = zengif::Limits::default();

    let gif_data = zengif::encode_gif(
        alloc::vec![frame],
        width,
        height,
        config,
        limits,
        &enough::Unstoppable,
    )
    .map_err(|e| CodecError::from_codec(ImageFormat::Gif, e))?;

    Ok(EncodeOutput {
        data: gif_data,
        format: ImageFormat::Gif,
    })
}
