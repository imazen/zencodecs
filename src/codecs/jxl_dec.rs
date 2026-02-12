//! JXL decode adapter — delegates to zenjxl.

use crate::{CodecError, DecodeOutput, ImageFormat, ImageInfo, Limits, Stop};

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

    Ok(DecodeOutput::new(result.pixels, convert_info(&result.info)))
}

fn convert_info(info: &zenjxl::JxlInfo) -> ImageInfo {
    let mut ii = ImageInfo::new(info.width, info.height, ImageFormat::Jxl)
        .with_alpha(info.has_alpha)
        .with_animation(info.has_animation);
    if let Some(ref icc) = info.icc_profile {
        ii = ii.with_icc_profile(icc.clone());
    }
    ii
}
