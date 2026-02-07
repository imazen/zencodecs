//! Image metadata probing without full decode.

use alloc::vec::Vec;

use crate::{CodecError, CodecRegistry, ImageFormat};

/// Unified image metadata from header parsing.
///
/// Obtained by probing an image file without decoding pixels.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ImageInfo {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Detected image format.
    pub format: ImageFormat,
    /// Whether the image has an alpha channel.
    pub has_alpha: bool,
    /// Whether the image contains animation (multiple frames).
    pub has_animation: bool,
    /// Number of frames (None if unknown without full parse).
    pub frame_count: Option<u32>,
    /// Embedded ICC color profile.
    pub icc_profile: Option<Vec<u8>>,
}

impl ImageInfo {
    /// Probe image metadata without decoding pixels.
    ///
    /// Uses format auto-detection and dispatches to the appropriate codec's probe.
    /// All compiled-in codecs are attempted.
    pub fn from_bytes(data: &[u8]) -> Result<Self, CodecError> {
        Self::from_bytes_with_registry(data, &CodecRegistry::all())
    }

    /// Probe image metadata with a specific registry.
    ///
    /// Only formats enabled in the registry will be attempted.
    pub fn from_bytes_with_registry(
        data: &[u8],
        registry: &CodecRegistry,
    ) -> Result<Self, CodecError> {
        // Detect format from magic bytes
        let format = ImageFormat::detect(data).ok_or(CodecError::UnrecognizedFormat)?;

        // Check if format is enabled in registry
        if !registry.can_decode(format) {
            return Err(CodecError::DisabledFormat(format));
        }

        // Dispatch to codec-specific probe
        Self::probe_format(data, format)
    }

    /// Probe with a known format (skips auto-detection).
    pub fn from_bytes_format(data: &[u8], format: ImageFormat) -> Result<Self, CodecError> {
        Self::probe_format(data, format)
    }

    /// Dispatch to format-specific probe implementation.
    fn probe_format(data: &[u8], format: ImageFormat) -> Result<Self, CodecError> {
        match format {
            #[cfg(feature = "jpeg")]
            ImageFormat::Jpeg => Self::probe_jpeg(data),
            #[cfg(not(feature = "jpeg"))]
            ImageFormat::Jpeg => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "webp")]
            ImageFormat::WebP => Self::probe_webp(data),
            #[cfg(not(feature = "webp"))]
            ImageFormat::WebP => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "gif")]
            ImageFormat::Gif => Self::probe_gif(data),
            #[cfg(not(feature = "gif"))]
            ImageFormat::Gif => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "png")]
            ImageFormat::Png => Self::probe_png(data),
            #[cfg(not(feature = "png"))]
            ImageFormat::Png => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "avif-decode")]
            ImageFormat::Avif => Self::probe_avif(data),
            #[cfg(not(feature = "avif-decode"))]
            ImageFormat::Avif => Err(CodecError::UnsupportedFormat(format)),
        }
    }

    // Codec-specific probe implementations (stubs for now, will be implemented with adapters)

    #[cfg(feature = "jpeg")]
    fn probe_jpeg(_data: &[u8]) -> Result<Self, CodecError> {
        // TODO: implement with zenjpeg adapter
        Err(CodecError::UnsupportedOperation {
            format: ImageFormat::Jpeg,
            detail: "probe not yet implemented",
        })
    }

    #[cfg(feature = "webp")]
    fn probe_webp(_data: &[u8]) -> Result<Self, CodecError> {
        // TODO: implement with zenwebp adapter
        Err(CodecError::UnsupportedOperation {
            format: ImageFormat::WebP,
            detail: "probe not yet implemented",
        })
    }

    #[cfg(feature = "gif")]
    fn probe_gif(_data: &[u8]) -> Result<Self, CodecError> {
        // TODO: implement with zengif adapter
        Err(CodecError::UnsupportedOperation {
            format: ImageFormat::Gif,
            detail: "probe not yet implemented",
        })
    }

    #[cfg(feature = "png")]
    fn probe_png(_data: &[u8]) -> Result<Self, CodecError> {
        // TODO: implement with png adapter
        Err(CodecError::UnsupportedOperation {
            format: ImageFormat::Png,
            detail: "probe not yet implemented",
        })
    }

    #[cfg(feature = "avif-decode")]
    fn probe_avif(_data: &[u8]) -> Result<Self, CodecError> {
        // TODO: implement with zenavif adapter
        Err(CodecError::UnsupportedOperation {
            format: ImageFormat::Avif,
            detail: "probe not yet implemented",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrecognized_format() {
        let data = b"not an image";
        let result = ImageInfo::from_bytes(data);
        assert!(matches!(result, Err(CodecError::UnrecognizedFormat)));
    }

    #[test]
    fn disabled_format() {
        let jpeg_data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        let registry = CodecRegistry::none(); // Nothing enabled

        let result = ImageInfo::from_bytes_with_registry(&jpeg_data, &registry);
        assert!(matches!(result, Err(CodecError::DisabledFormat(_))));
    }
}
