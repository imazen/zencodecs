//! Image metadata probing without full decode.

pub use zencodec_types::ImageInfo;

use crate::{CodecError, CodecRegistry, ImageFormat};

/// Probe partial image data for metadata without decoding pixels.
///
/// Unlike [`from_bytes`], this works with truncated data (e.g., first N bytes
/// from an HTTP range request). Missing dimensions result in `None` fields
/// rather than an error. All compiled-in codecs are attempted.
///
/// Uses pure byte parsing — no codec crate dependencies. Works even if a
/// codec feature isn't compiled in.
pub fn probe(data: &[u8]) -> Result<crate::ProbeResult, CodecError> {
    let format = ImageFormat::detect(data).ok_or(CodecError::UnrecognizedFormat)?;
    Ok(crate::ProbeResult::for_format(data, format))
}

/// Probe partial image data with a specific registry.
///
/// Only formats enabled in the registry will be accepted.
pub fn probe_with_registry(
    data: &[u8],
    registry: &CodecRegistry,
) -> Result<crate::ProbeResult, CodecError> {
    let format = ImageFormat::detect(data).ok_or(CodecError::UnrecognizedFormat)?;
    if !registry.can_decode(format) {
        return Err(CodecError::DisabledFormat(format));
    }
    Ok(crate::ProbeResult::for_format(data, format))
}

/// Probe partial image data for a known format (skips auto-detection).
///
/// Never fails — insufficient data results in `None` fields.
pub fn probe_format(data: &[u8], format: ImageFormat) -> crate::ProbeResult {
    crate::ProbeResult::for_format(data, format)
}

/// Probe image metadata without decoding pixels.
///
/// Uses format auto-detection and dispatches to the appropriate codec's probe.
/// All compiled-in codecs are attempted.
pub fn from_bytes(data: &[u8]) -> Result<ImageInfo, CodecError> {
    from_bytes_with_registry(data, &CodecRegistry::all())
}

/// Probe image metadata with a specific registry.
///
/// Only formats enabled in the registry will be attempted.
pub fn from_bytes_with_registry(
    data: &[u8],
    registry: &CodecRegistry,
) -> Result<ImageInfo, CodecError> {
    let format = ImageFormat::detect(data).ok_or(CodecError::UnrecognizedFormat)?;

    if !registry.can_decode(format) {
        return Err(CodecError::DisabledFormat(format));
    }

    probe_format_full(data, format)
}

/// Probe with a known format (skips auto-detection).
pub fn from_bytes_format(data: &[u8], format: ImageFormat) -> Result<ImageInfo, CodecError> {
    probe_format_full(data, format)
}

/// Compute actual decode output dimensions for an image.
///
/// Unlike [`from_bytes`] which returns stored file dimensions, this applies
/// codec-specific config transforms (JPEG DctScale, auto_orient) to predict
/// what [`DecodeRequest::decode`](crate::DecodeRequest::decode) will actually produce.
///
/// For codecs without dimension transforms, this is equivalent to `from_bytes`.
pub fn decode_info(data: &[u8]) -> Result<ImageInfo, CodecError> {
    decode_info_with_config(data, None)
}

/// Compute actual decode output dimensions with codec-specific config.
pub fn decode_info_with_config(
    data: &[u8],
    codec_config: Option<&crate::config::CodecConfig>,
) -> Result<ImageInfo, CodecError> {
    let format = ImageFormat::detect(data).ok_or(CodecError::UnrecognizedFormat)?;
    decode_info_format(data, format, codec_config)
}

/// Compute actual decode output dimensions for a known format.
fn decode_info_format(
    data: &[u8],
    format: ImageFormat,
    codec_config: Option<&crate::config::CodecConfig>,
) -> Result<ImageInfo, CodecError> {
    match format {
        #[cfg(feature = "jpeg")]
        ImageFormat::Jpeg => crate::codecs::jpeg::decode_info(data, codec_config),

        // Other codecs: decode_info == probe (no dimension transforms)
        _ => probe_format_full(data, format),
    }
}

/// Dispatch to format-specific full probe (requires codec feature).
fn probe_format_full(data: &[u8], format: ImageFormat) -> Result<ImageInfo, CodecError> {
    match format {
        #[cfg(feature = "jpeg")]
        ImageFormat::Jpeg => crate::codecs::jpeg::probe(data),
        #[cfg(not(feature = "jpeg"))]
        ImageFormat::Jpeg => Err(CodecError::UnsupportedFormat(format)),

        #[cfg(feature = "webp")]
        ImageFormat::WebP => crate::codecs::webp::probe(data),
        #[cfg(not(feature = "webp"))]
        ImageFormat::WebP => Err(CodecError::UnsupportedFormat(format)),

        #[cfg(feature = "gif")]
        ImageFormat::Gif => crate::codecs::gif::probe(data),
        #[cfg(not(feature = "gif"))]
        ImageFormat::Gif => Err(CodecError::UnsupportedFormat(format)),

        #[cfg(feature = "png")]
        ImageFormat::Png => crate::codecs::png::probe(data),
        #[cfg(not(feature = "png"))]
        ImageFormat::Png => Err(CodecError::UnsupportedFormat(format)),

        #[cfg(feature = "avif-decode")]
        ImageFormat::Avif => crate::codecs::avif_dec::probe(data),
        #[cfg(not(feature = "avif-decode"))]
        ImageFormat::Avif => Err(CodecError::UnsupportedFormat(format)),

        #[cfg(feature = "jxl-decode")]
        ImageFormat::Jxl => crate::codecs::jxl_dec::probe(data),
        #[cfg(not(feature = "jxl-decode"))]
        ImageFormat::Jxl => Err(CodecError::UnsupportedFormat(format)),

        _ => Err(CodecError::UnsupportedFormat(format)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrecognized_format() {
        let data = b"not an image";
        let result = from_bytes(data);
        assert!(matches!(result, Err(CodecError::UnrecognizedFormat)));
    }

    #[test]
    fn disabled_format() {
        let jpeg_data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        let registry = CodecRegistry::none();

        let result = from_bytes_with_registry(&jpeg_data, &registry);
        assert!(matches!(result, Err(CodecError::DisabledFormat(_))));
    }
}
