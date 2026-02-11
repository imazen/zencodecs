//! JXL decode adapter — delegates to zenjxl.

use crate::{CodecError, DecodeOutput, ImageFormat, ImageInfo, Limits, PixelData, Stop};

/// Probe JXL metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let info = zenjxl::probe(data).map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))?;
    Ok(convert_info(&info))
}

/// Decode JXL to pixels.
pub(crate) fn decode(
    data: &[u8],
    limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let jxl_limits = limits.map(|lim| zenjxl::JxlLimits {
        max_pixels: lim.max_pixels,
        max_memory_bytes: lim.max_memory_bytes,
    });

    let result = zenjxl::decode(data, jxl_limits.as_ref())
        .map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))?;

    Ok(DecodeOutput {
        pixels: convert_pixels(result.pixels),
        info: convert_info(&result.info),
        #[cfg(feature = "jpeg")]
        jpeg_extras: None,
    })
}

fn convert_info(info: &zenjxl::JxlInfo) -> ImageInfo {
    ImageInfo {
        width: info.width,
        height: info.height,
        format: ImageFormat::Jxl,
        has_alpha: info.has_alpha,
        has_animation: info.has_animation,
        frame_count: None,
        icc_profile: info.icc_profile.clone(),
        exif: None,
        xmp: None,
    }
}

fn convert_pixels(pixels: zencodec_types::PixelData) -> PixelData {
    match pixels {
        zencodec_types::PixelData::Rgb8(img) => PixelData::Rgb8(img),
        zencodec_types::PixelData::Rgba8(img) => PixelData::Rgba8(img),
        zencodec_types::PixelData::Rgb16(img) => PixelData::Rgb16(img),
        zencodec_types::PixelData::Rgba16(img) => PixelData::Rgba16(img),
        zencodec_types::PixelData::RgbF32(img) => PixelData::RgbF32(img),
        zencodec_types::PixelData::RgbaF32(img) => PixelData::RgbaF32(img),
        zencodec_types::PixelData::Gray8(img) => PixelData::Gray8(img),
        _ => unreachable!("unknown PixelData variant"),
    }
}
