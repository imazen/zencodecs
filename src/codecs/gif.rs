//! GIF codec adapter using zengif.

use crate::{
    CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, Limits, PixelLayout, Stop,
};

/// Probe GIF metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let limits = zengif::Limits::default();

    // Use decode_gif to get metadata (TODO: optimize to avoid full decode)
    let (metadata, _frames, _stats) = zengif::decode_gif(data, limits, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Gif, e))?;

    Ok(ImageInfo {
        width: metadata.width as u32,
        height: metadata.height as u32,
        format: ImageFormat::Gif,
        has_alpha: true, // GIF supports transparency
        has_animation: metadata.frame_count > 1,
        frame_count: Some(metadata.frame_count as u32),
        icc_profile: None, // GIF doesn't typically have ICC profiles
    })
}

/// Decode GIF to pixels.
///
/// For animated GIFs, this returns only the first frame.
/// TODO: Add animation support via streaming decoder.
pub(crate) fn decode(
    data: &[u8],
    _output_layout: PixelLayout,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let limits = zengif::Limits::default();

    // Decode all frames
    let (metadata, mut frames, _stats) = zengif::decode_gif(data, limits, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Gif, e))?;

    if frames.is_empty() {
        return Err(CodecError::InvalidInput("GIF has no frames".into()));
    }

    // Take first frame
    let frame = frames.remove(0);

    // Convert Vec<Rgba> to Vec<u8> (zengif returns Vec<Rgba>)
    let pixels = frame
        .pixels
        .into_iter()
        .flat_map(|rgba| [rgba.r, rgba.g, rgba.b, rgba.a])
        .collect();

    Ok(DecodeOutput {
        pixels,
        width: metadata.width as u32,
        height: metadata.height as u32,
        layout: PixelLayout::Rgba8, // zengif always returns RGBA
        info: ImageInfo {
            width: metadata.width as u32,
            height: metadata.height as u32,
            format: ImageFormat::Gif,
            has_alpha: true,
            has_animation: metadata.frame_count > 1,
            frame_count: Some(metadata.frame_count as u32),
            icc_profile: None,
        },
    })
}

/// Encode pixels to GIF.
///
/// Creates a single-frame GIF. For animation support, use zengif directly.
#[allow(clippy::too_many_arguments)]
pub(crate) fn encode(
    pixels: &[u8],
    width: u32,
    height: u32,
    layout: PixelLayout,
    _quality: Option<f32>, // GIF doesn't have quality setting
    _lossless: bool,       // GIF is always lossless (after quantization)
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    // Convert dimensions to u16 (GIF limitation)
    let width_u16 = width
        .try_into()
        .map_err(|_| CodecError::InvalidInput("width exceeds GIF maximum (65535)".into()))?;
    let height_u16 = height
        .try_into()
        .map_err(|_| CodecError::InvalidInput("height exceeds GIF maximum (65535)".into()))?;

    // Convert to RGBA bytes, then use zengif's from_bytes constructor
    let rgba_bytes = crate::pixel::to_rgba(pixels, layout);

    // Create frame input (zengif's from_bytes takes RGBA &[u8] directly)
    let frame = zengif::FrameInput::from_bytes(width_u16, height_u16, 10, &rgba_bytes); // 10 = 100ms delay

    // Create encoder config
    let config = zengif::EncoderConfig::new();
    let limits = zengif::Limits::default();

    // Encode single frame
    let gif_data = zengif::encode_gif(
        alloc::vec![frame],
        width_u16,
        height_u16,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_limits() {
        // GIF uses u16 for dimensions
        let result = encode(&[], 70000, 100, PixelLayout::Rgba8, None, false, None, None);
        assert!(matches!(result, Err(CodecError::InvalidInput(_))));
    }
}
