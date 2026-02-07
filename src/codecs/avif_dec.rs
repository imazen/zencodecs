//! AVIF decode adapter using zenavif.

use alloc::vec::Vec;

use crate::{CodecError, DecodeOutput, ImageFormat, ImageInfo, Limits, PixelLayout, Stop};

/// Probe AVIF metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    // For probe, we need to decode the image to get full metadata
    // TODO: zenavif should provide a probe-only function that doesn't decode pixels
    let image = zenavif::decode(data).map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?;

    Ok(ImageInfo {
        width: image.width() as u32,
        height: image.height() as u32,
        format: ImageFormat::Avif,
        has_alpha: image.has_alpha(),
        has_animation: false, // TODO: zenavif doesn't expose animation metadata yet
        frame_count: Some(1),
        icc_profile: None, // TODO: zenavif doesn't expose ICC profile yet
    })
}

/// Decode AVIF to pixels.
pub(crate) fn decode(
    data: &[u8],
    output_layout: PixelLayout,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    // TODO: zenavif uses enough::Stop, but zencodecs has its own Stop trait
    // For now, just use the simple decode() without stop token support
    let image = zenavif::decode(data).map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?;

    let width = image.width() as u32;
    let height = image.height() as u32;
    let has_alpha = image.has_alpha();

    // Convert DecodedImage to flat byte array
    let (pixels, layout) = match image {
        zenavif::DecodedImage::Rgb8(img_vec) => {
            // ImgVec<Rgb<u8>> - convert to flat RGB bytes
            let (vec, _, _) = img_vec.into_contiguous_buf();
            // Use ComponentBytes trait to convert Vec<Rgb<u8>> to &[u8]
            use rgb::ComponentBytes;
            let bytes = vec.as_slice().as_bytes().to_vec();
            (bytes, PixelLayout::Rgb8)
        }
        zenavif::DecodedImage::Rgba8(img_vec) => {
            // ImgVec<Rgba<u8>> - convert to flat RGBA bytes
            let (vec, _, _) = img_vec.into_contiguous_buf();
            // Use ComponentBytes trait to convert Vec<Rgba<u8>> to &[u8]
            use rgb::ComponentBytes;
            let bytes = vec.as_slice().as_bytes().to_vec();
            (bytes, PixelLayout::Rgba8)
        }
        _ => {
            // For now, only support 8-bit RGB/RGBA
            return Err(CodecError::UnsupportedOperation {
                format: ImageFormat::Avif,
                detail: "only 8-bit RGB/RGBA output supported (16-bit/grayscale not yet implemented)",
            });
        }
    };

    // TODO: Convert to requested output_layout if different from decoded layout
    let _ = output_layout; // Suppress unused warning

    Ok(DecodeOutput {
        pixels,
        width,
        height,
        layout,
        info: ImageInfo {
            width,
            height,
            format: ImageFormat::Avif,
            has_alpha,
            has_animation: false,
            frame_count: Some(1),
            icc_profile: None,
        },
    })
}
