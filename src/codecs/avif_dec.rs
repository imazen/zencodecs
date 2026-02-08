//! AVIF decode adapter using zenavif.

use crate::config::CodecConfig;
use crate::{CodecError, DecodeOutput, ImageFormat, ImageInfo, Limits, PixelData, Stop};

/// Probe AVIF metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let image = zenavif::decode(data).map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?;

    Ok(ImageInfo {
        width: image.width() as u32,
        height: image.height() as u32,
        format: ImageFormat::Avif,
        has_alpha: image.has_alpha(),
        has_animation: false,
        frame_count: Some(1),
        icc_profile: None,
        exif: None,
        xmp: None,
    })
}

/// Decode AVIF to pixels.
pub(crate) fn decode(
    data: &[u8],
    _codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    // TODO: use codec_config.avif_decoder when zenavif supports DecoderConfig
    let image = zenavif::decode(data).map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))?;

    let width = image.width() as u32;
    let height = image.height() as u32;
    let has_alpha = image.has_alpha();

    let pixels = match image {
        zenavif::DecodedImage::Rgb8(img_vec) => PixelData::Rgb8(img_vec),
        zenavif::DecodedImage::Rgba8(img_vec) => PixelData::Rgba8(img_vec),
        zenavif::DecodedImage::Rgb16(img_vec) => PixelData::Rgb16(img_vec),
        zenavif::DecodedImage::Rgba16(img_vec) => PixelData::Rgba16(img_vec),
        zenavif::DecodedImage::Gray8(img_vec) => {
            let (buf, w, h) = img_vec.into_contiguous_buf();
            let gray: alloc::vec::Vec<rgb::Gray<u8>> = buf.into_iter().map(rgb::Gray).collect();
            PixelData::Gray8(imgref::ImgVec::new(gray, w, h))
        }
        _ => {
            return Err(CodecError::UnsupportedOperation {
                format: ImageFormat::Avif,
                detail: "unsupported pixel format from zenavif",
            });
        }
    };

    Ok(DecodeOutput {
        pixels,
        info: ImageInfo {
            width,
            height,
            format: ImageFormat::Avif,
            has_alpha,
            has_animation: false,
            frame_count: Some(1),
            icc_profile: None,
            exif: None,
            xmp: None,
        },
        #[cfg(feature = "jpeg")]
        jpeg_extras: None,
    })
}
