//! Image encoding.

use alloc::vec::Vec;

use crate::pixel::{ImgRef, Rgb, Rgba};
use crate::{CodecError, CodecRegistry, ImageFormat, ImageMetadata, Limits, Stop};

/// Encoded image output.
#[derive(Clone, Debug)]
pub struct EncodeOutput {
    /// Encoded image data.
    pub data: Vec<u8>,
    /// Format used for encoding (useful when auto-selected).
    pub format: ImageFormat,
}

/// Image encode request builder.
///
/// # Example
///
/// ```no_run
/// use zencodecs::{EncodeRequest, ImageFormat};
/// use zencodecs::pixel::{ImgVec, Rgba};
///
/// let pixels = ImgVec::new(vec![Rgba { r: 0u8, g: 0, b: 0, a: 255 }; 100*100], 100, 100);
/// let output = EncodeRequest::new(ImageFormat::WebP)
///     .with_quality(85.0)
///     .encode_rgba8(pixels.as_ref())?;
/// # Ok::<(), zencodecs::CodecError>(())
/// ```
pub struct EncodeRequest<'a> {
    format: Option<ImageFormat>,
    quality: Option<f32>,
    effort: Option<u32>,
    lossless: bool,
    limits: Option<&'a Limits>,
    stop: Option<&'a dyn Stop>,
    metadata: Option<&'a ImageMetadata<'a>>,
    registry: Option<&'a CodecRegistry>,
}

impl<'a> EncodeRequest<'a> {
    /// Encode to a specific format.
    pub fn new(format: ImageFormat) -> Self {
        Self {
            format: Some(format),
            quality: None,
            effort: None,
            lossless: false,
            limits: None,
            stop: None,
            metadata: None,
            registry: None,
        }
    }

    /// Auto-select best format based on image stats and allowed encoders.
    pub fn auto() -> Self {
        Self {
            format: None,
            quality: None,
            effort: None,
            lossless: false,
            limits: None,
            stop: None,
            metadata: None,
            registry: None,
        }
    }

    /// Set quality (0-100).
    pub fn with_quality(mut self, quality: f32) -> Self {
        self.quality = Some(quality);
        self
    }

    /// Set encoding effort (speed/quality tradeoff).
    pub fn with_effort(mut self, effort: u32) -> Self {
        self.effort = Some(effort);
        self
    }

    /// Request lossless encoding.
    pub fn with_lossless(mut self, lossless: bool) -> Self {
        self.lossless = lossless;
        self
    }

    /// Set resource limits.
    pub fn with_limits(mut self, limits: &'a Limits) -> Self {
        self.limits = Some(limits);
        self
    }

    /// Set a cancellation token.
    pub fn with_stop(mut self, stop: &'a dyn Stop) -> Self {
        self.stop = Some(stop);
        self
    }

    /// Set metadata to embed in the output.
    pub fn with_metadata(mut self, metadata: &'a ImageMetadata<'a>) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set a codec registry to control which formats are enabled.
    pub fn with_registry(mut self, registry: &'a CodecRegistry) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Encode RGB8 pixels.
    pub fn encode_rgb8(self, img: ImgRef<Rgb<u8>>) -> Result<EncodeOutput, CodecError> {
        let default_registry = CodecRegistry::all();
        let registry = self.registry.unwrap_or(&default_registry);
        let has_alpha = false;

        let format = match self.format {
            Some(f) => f,
            None => self.auto_select_format(has_alpha, registry)?,
        };

        self.validate_and_dispatch_rgb8(format, img, registry)
    }

    /// Encode RGBA8 pixels.
    pub fn encode_rgba8(self, img: ImgRef<Rgba<u8>>) -> Result<EncodeOutput, CodecError> {
        let default_registry = CodecRegistry::all();
        let registry = self.registry.unwrap_or(&default_registry);

        // Check if any pixel actually has transparency
        let has_alpha = img.pixels().any(|p| p.a < 255);

        let format = match self.format {
            Some(f) => f,
            None => self.auto_select_format(has_alpha, registry)?,
        };

        self.validate_and_dispatch_rgba8(format, img, registry)
    }

    fn validate_and_dispatch_rgb8(
        self,
        format: ImageFormat,
        img: ImgRef<Rgb<u8>>,
        registry: &CodecRegistry,
    ) -> Result<EncodeOutput, CodecError> {
        if !registry.can_encode(format) {
            return Err(CodecError::DisabledFormat(format));
        }
        if self.lossless && !format.supports_lossless() {
            return Err(CodecError::UnsupportedOperation {
                format,
                detail: "lossless encoding not supported",
            });
        }
        self.encode_format_rgb8(format, img)
    }

    fn validate_and_dispatch_rgba8(
        self,
        format: ImageFormat,
        img: ImgRef<Rgba<u8>>,
        registry: &CodecRegistry,
    ) -> Result<EncodeOutput, CodecError> {
        if !registry.can_encode(format) {
            return Err(CodecError::DisabledFormat(format));
        }
        if self.lossless && !format.supports_lossless() {
            return Err(CodecError::UnsupportedOperation {
                format,
                detail: "lossless encoding not supported",
            });
        }
        self.encode_format_rgba8(format, img)
    }

    /// Auto-select format.
    fn auto_select_format(
        &self,
        has_alpha: bool,
        registry: &CodecRegistry,
    ) -> Result<ImageFormat, CodecError> {
        if self.lossless {
            if registry.can_encode(ImageFormat::WebP) {
                return Ok(ImageFormat::WebP);
            }
            if registry.can_encode(ImageFormat::Png) {
                return Ok(ImageFormat::Png);
            }
        } else if has_alpha {
            if registry.can_encode(ImageFormat::WebP) {
                return Ok(ImageFormat::WebP);
            }
            if registry.can_encode(ImageFormat::Avif) {
                return Ok(ImageFormat::Avif);
            }
            if registry.can_encode(ImageFormat::Png) {
                return Ok(ImageFormat::Png);
            }
        } else {
            if registry.can_encode(ImageFormat::Jpeg) {
                return Ok(ImageFormat::Jpeg);
            }
            if registry.can_encode(ImageFormat::WebP) {
                return Ok(ImageFormat::WebP);
            }
            if registry.can_encode(ImageFormat::Avif) {
                return Ok(ImageFormat::Avif);
            }
        }

        Err(CodecError::NoSuitableEncoder)
    }

    fn encode_format_rgb8(
        self,
        format: ImageFormat,
        img: ImgRef<Rgb<u8>>,
    ) -> Result<EncodeOutput, CodecError> {
        match format {
            #[cfg(feature = "jpeg")]
            ImageFormat::Jpeg => {
                crate::codecs::jpeg::encode_rgb8(img, self.quality, self.limits, self.stop)
            }
            #[cfg(not(feature = "jpeg"))]
            ImageFormat::Jpeg => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "webp")]
            ImageFormat::WebP => crate::codecs::webp::encode_rgb8(
                img,
                self.quality,
                self.lossless,
                self.limits,
                self.stop,
            ),
            #[cfg(not(feature = "webp"))]
            ImageFormat::WebP => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "gif")]
            ImageFormat::Gif => crate::codecs::gif::encode_rgb8(img, self.limits, self.stop),
            #[cfg(not(feature = "gif"))]
            ImageFormat::Gif => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "png")]
            ImageFormat::Png => crate::codecs::png::encode_rgb8(img, self.limits, self.stop),
            #[cfg(not(feature = "png"))]
            ImageFormat::Png => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "avif-encode")]
            ImageFormat::Avif => {
                crate::codecs::avif_enc::encode_rgb8(img, self.quality, self.limits, self.stop)
            }
            #[cfg(not(feature = "avif-encode"))]
            ImageFormat::Avif => Err(CodecError::UnsupportedFormat(format)),
        }
    }

    fn encode_format_rgba8(
        self,
        format: ImageFormat,
        img: ImgRef<Rgba<u8>>,
    ) -> Result<EncodeOutput, CodecError> {
        match format {
            #[cfg(feature = "jpeg")]
            ImageFormat::Jpeg => {
                crate::codecs::jpeg::encode_rgba8(img, self.quality, self.limits, self.stop)
            }
            #[cfg(not(feature = "jpeg"))]
            ImageFormat::Jpeg => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "webp")]
            ImageFormat::WebP => crate::codecs::webp::encode_rgba8(
                img,
                self.quality,
                self.lossless,
                self.limits,
                self.stop,
            ),
            #[cfg(not(feature = "webp"))]
            ImageFormat::WebP => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "gif")]
            ImageFormat::Gif => crate::codecs::gif::encode_rgba8(img, self.limits, self.stop),
            #[cfg(not(feature = "gif"))]
            ImageFormat::Gif => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "png")]
            ImageFormat::Png => crate::codecs::png::encode_rgba8(img, self.limits, self.stop),
            #[cfg(not(feature = "png"))]
            ImageFormat::Png => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "avif-encode")]
            ImageFormat::Avif => {
                crate::codecs::avif_enc::encode_rgba8(img, self.quality, self.limits, self.stop)
            }
            #[cfg(not(feature = "avif-encode"))]
            ImageFormat::Avif => Err(CodecError::UnsupportedFormat(format)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_pattern() {
        let request = EncodeRequest::new(ImageFormat::Jpeg)
            .with_quality(85.0)
            .with_lossless(false);

        assert_eq!(request.format, Some(ImageFormat::Jpeg));
        assert_eq!(request.quality, Some(85.0));
        assert!(!request.lossless);
    }

    #[test]
    fn lossless_with_jpeg_error() {
        let img = imgref::ImgVec::new(
            vec![
                Rgb {
                    r: 255u8,
                    g: 255,
                    b: 255
                };
                100 * 100
            ],
            100,
            100,
        );
        let result = EncodeRequest::new(ImageFormat::Jpeg)
            .with_lossless(true)
            .encode_rgb8(img.as_ref());

        assert!(matches!(
            result,
            Err(CodecError::UnsupportedOperation { .. })
        ));
    }
}
