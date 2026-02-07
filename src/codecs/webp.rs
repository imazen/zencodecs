//! WebP codec adapter using zenwebp.

use crate::{
    CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, Limits, PixelLayout, Stop,
};

/// Convert zencodecs PixelLayout to zenwebp PixelLayout.
fn to_webp_layout(layout: PixelLayout) -> zenwebp::PixelLayout {
    match layout {
        PixelLayout::Rgb8 => zenwebp::PixelLayout::Rgb8,
        PixelLayout::Rgba8 => zenwebp::PixelLayout::Rgba8,
        PixelLayout::Bgr8 => zenwebp::PixelLayout::Bgr8,
        PixelLayout::Bgra8 => zenwebp::PixelLayout::Bgra8,
    }
}

/// Probe WebP metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let decoder = zenwebp::WebPDecoder::new(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;

    let (width, height) = decoder.dimensions();
    let has_alpha = decoder.has_alpha();

    // Check if it's animated
    let demuxer = zenwebp::WebPDemuxer::new(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
    let has_animation = demuxer.is_animated();
    let frame_count = demuxer.num_frames();

    // Get ICC profile if present
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
    output_layout: PixelLayout,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    // Decode based on requested layout
    let (pixels, width, height) = match output_layout {
        PixelLayout::Rgb8 => zenwebp::decode_rgb(data)
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?,
        PixelLayout::Rgba8 => zenwebp::decode_rgba(data)
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?,
        PixelLayout::Bgr8 => zenwebp::decode_bgr(data)
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?,
        PixelLayout::Bgra8 => zenwebp::decode_bgra(data)
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?,
    };

    // Get additional info
    let decoder = zenwebp::WebPDecoder::new(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
    let has_alpha = decoder.has_alpha();

    // Check animation status
    let demuxer = zenwebp::WebPDemuxer::new(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?;
    let has_animation = demuxer.is_animated();
    let icc_profile = demuxer.icc_profile().map(|p| p.to_vec());

    Ok(DecodeOutput {
        pixels,
        width,
        height,
        layout: output_layout,
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

/// Encode pixels to WebP.
#[allow(clippy::too_many_arguments)]
pub(crate) fn encode(
    pixels: &[u8],
    width: u32,
    height: u32,
    layout: PixelLayout,
    quality: Option<f32>,
    lossless: bool,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let webp_layout = to_webp_layout(layout);

    let webp_data = if lossless {
        // Lossless encoding
        let config = zenwebp::LosslessConfig::new();
        zenwebp::EncodeRequest::lossless(&config, pixels, webp_layout, width, height)
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    } else {
        // Lossy encoding
        let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0);
        let config = zenwebp::LossyConfig::new().with_quality(quality);
        zenwebp::EncodeRequest::lossy(&config, pixels, webp_layout, width, height)
            .encode()
            .map_err(|e| CodecError::from_codec(ImageFormat::WebP, e))?
    };

    Ok(EncodeOutput {
        data: webp_data,
        format: ImageFormat::WebP,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_layout_conversion() {
        // Verify all formats can be converted
        for layout in [
            PixelLayout::Rgb8,
            PixelLayout::Rgba8,
            PixelLayout::Bgr8,
            PixelLayout::Bgra8,
        ] {
            let _webp_layout = to_webp_layout(layout);
        }
    }
}
