//! GIF codec adapter using zengif.

use crate::config::CodecConfig;
use crate::pixel::{ImgRef, ImgVec, Rgb, Rgba};
use crate::{
    CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, Limits, PixelData, Stop,
};

/// Probe GIF metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let limits = zengif::Limits::default();

    let (metadata, _frames, _stats) = zengif::decode_gif(data, limits, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Gif, e))?;

    Ok(ImageInfo {
        width: metadata.width as u32,
        height: metadata.height as u32,
        format: ImageFormat::Gif,
        has_alpha: true,
        has_animation: metadata.frame_count > 1,
        frame_count: Some(metadata.frame_count as u32),
        icc_profile: None,
        exif: None,
        xmp: None,
    })
}

/// Decode GIF to pixels (first frame only).
pub(crate) fn decode(
    data: &[u8],
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let limits = zengif::Limits::default();

    let (metadata, mut frames, _stats) = zengif::decode_gif(data, limits, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Gif, e))?;

    if frames.is_empty() {
        return Err(CodecError::InvalidInput("GIF has no frames".into()));
    }

    let frame = frames.remove(0);
    let width = metadata.width as usize;
    let height = metadata.height as usize;

    let rgba_pixels: alloc::vec::Vec<Rgba<u8>> = frame
        .pixels
        .into_iter()
        .map(|p| Rgba {
            r: p.r,
            g: p.g,
            b: p.b,
            a: p.a,
        })
        .collect();

    let img = ImgVec::new(rgba_pixels, width, height);

    Ok(DecodeOutput {
        pixels: PixelData::Rgba8(img),
        info: ImageInfo {
            width: width as u32,
            height: height as u32,
            format: ImageFormat::Gif,
            has_alpha: true,
            has_animation: metadata.frame_count > 1,
            frame_count: Some(metadata.frame_count as u32),
            icc_profile: None,
            exif: None,
            xmp: None,
        },
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
        enough::Unstoppable,
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
    let rgba_bytes: alloc::vec::Vec<u8> = buf.iter().flat_map(|p| [p.r, p.g, p.b, p.a]).collect();

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
        enough::Unstoppable,
    )
    .map_err(|e| CodecError::from_codec(ImageFormat::Gif, e))?;

    Ok(EncodeOutput {
        data: gif_data,
        format: ImageFormat::Gif,
    })
}
