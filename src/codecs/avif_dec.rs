//! AVIF decode adapter using zenavif.

use crate::config::CodecConfig;
use crate::{CodecError, DecodeOutput, ImageFormat, ImageInfo, Limits, Stop};

/// Probe AVIF metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let image = zenavif::decode(data).map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?;

    Ok(
        ImageInfo::new(image.width(), image.height(), ImageFormat::Avif)
            .with_alpha(image.has_alpha())
            .with_frame_count(1),
    )
}

/// Decode AVIF to pixels.
pub(crate) fn decode(
    data: &[u8],
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let stop = crate::limits::stop_or_default(stop);

    // Build zenavif DecoderConfig from codec_config or defaults
    let mut avif_config = codec_config
        .and_then(|c| c.avif_decoder.as_ref())
        .map(|c| c.as_ref().clone())
        .unwrap_or_default();

    // Apply frame_size_limit from zencodecs Limits
    if let Some(lim) = limits {
        if let Some(max_px) = lim.max_pixels {
            avif_config = avif_config.frame_size_limit(max_px.min(u32::MAX as u64) as u32);
        }
    }

    let image = zenavif::decode_with(data, &avif_config, &stop)
        .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?;

    let width = image.width() as u32;
    let height = image.height() as u32;
    let has_alpha = image.has_alpha();

    // Check dimension limits
    if let Some(lim) = limits {
        let bpp: u32 = if has_alpha { 4 } else { 3 };
        lim.validate(width, height, bpp)?;
    }

    // zenavif returns zencodec_types::PixelData, which is our PixelData
    let pixels = image;

    Ok(DecodeOutput::new(
        pixels,
        ImageInfo::new(width, height, ImageFormat::Avif)
            .with_alpha(has_alpha)
            .with_frame_count(1),
    ))
}
