//! Image encoding.

use alloc::vec::Vec;

use crate::{CodecError, CodecRegistry, ImageFormat, ImageMetadata, Limits, PixelLayout, Stop};

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
/// Use this to encode pixels to various image formats.
///
/// # Example
///
/// ```no_run
/// use zencodecs::{EncodeRequest, ImageFormat, PixelLayout};
///
/// let pixels: &[u8] = &[]; // your pixel data
/// let output = EncodeRequest::new(ImageFormat::WebP)
///     .with_quality(85.0)
///     .encode(pixels, 100, 100, PixelLayout::Rgba8)?;
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
    ///
    /// Uses the registry to determine which formats are candidates.
    /// The selection logic is simple and deterministic (not optimal).
    pub fn auto() -> Self {
        Self {
            format: None, // Will be selected during encode()
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
    ///
    /// Mapped to each codec's native scale. Not all formats support this
    /// (GIF and lossless PNG ignore it).
    pub fn with_quality(mut self, quality: f32) -> Self {
        self.quality = Some(quality);
        self
    }

    /// Set encoding effort (speed/quality tradeoff).
    ///
    /// Higher effort = slower encoding, better quality/compression.
    /// Exact interpretation varies by codec.
    pub fn with_effort(mut self, effort: u32) -> Self {
        self.effort = Some(effort);
        self
    }

    /// Request lossless encoding.
    ///
    /// Only formats that support lossless will be used.
    /// For lossy-only formats, returns an error.
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
    ///
    /// Required when using `auto()` to restrict candidates.
    /// If not set, all compiled-in codecs are available.
    pub fn with_registry(mut self, registry: &'a CodecRegistry) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Encode pixels to the target format.
    pub fn encode(
        self,
        pixels: &[u8],
        width: u32,
        height: u32,
        layout: PixelLayout,
    ) -> Result<EncodeOutput, CodecError> {
        // Get the registry (default to all if not specified)
        let default_registry = CodecRegistry::all();
        let registry = self.registry.unwrap_or(&default_registry);

        // Determine format (auto-select if needed)
        let format = match self.format {
            Some(f) => f,
            None => self.auto_select_format(pixels, width, height, layout, registry)?,
        };

        // Check if format is enabled
        if !registry.can_encode(format) {
            return Err(CodecError::DisabledFormat(format));
        }

        // Check if lossless is requested but format doesn't support it
        if self.lossless && !format.supports_lossless() {
            return Err(CodecError::UnsupportedOperation {
                format,
                detail: "lossless encoding not supported",
            });
        }

        // Validate input buffer size
        let expected_size = layout
            .min_buffer_size(width, height)
            .ok_or_else(|| CodecError::InvalidInput("dimensions too large".into()))?;

        if pixels.len() < expected_size {
            return Err(CodecError::InvalidInput("pixel buffer too small".into()));
        }

        // Dispatch to format-specific encoder
        self.encode_format(format, pixels, width, height, layout)
    }

    /// Encode into a pre-allocated buffer.
    ///
    /// The buffer will be resized to fit the encoded data.
    pub fn encode_into(
        self,
        pixels: &[u8],
        width: u32,
        height: u32,
        layout: PixelLayout,
        output: &mut Vec<u8>,
    ) -> Result<EncodeOutput, CodecError> {
        let result = self.encode(pixels, width, height, layout)?;
        output.clear();
        output.extend_from_slice(&result.data);
        Ok(EncodeOutput {
            data: output.clone(),
            format: result.format,
        })
    }

    /// Auto-select format based on image characteristics.
    fn auto_select_format(
        &self,
        pixels: &[u8],
        _width: u32,
        _height: u32,
        layout: PixelLayout,
        registry: &CodecRegistry,
    ) -> Result<ImageFormat, CodecError> {
        // TODO: implement smart selection based on image stats
        // For now, simple fallback logic

        if self.lossless {
            // Prefer lossless formats
            if registry.can_encode(ImageFormat::WebP) {
                return Ok(ImageFormat::WebP);
            }
            if registry.can_encode(ImageFormat::Png) {
                return Ok(ImageFormat::Png);
            }
        } else {
            // Prefer lossy formats
            let has_alpha = layout.has_alpha() && self.has_alpha_pixels(pixels, layout);

            if has_alpha {
                // Can't use JPEG with alpha
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
                // No alpha, can use any format
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
        }

        Err(CodecError::NoSuitableEncoder)
    }

    /// Quick check if pixels have any transparency.
    fn has_alpha_pixels(&self, pixels: &[u8], layout: PixelLayout) -> bool {
        if !layout.has_alpha() {
            return false;
        }

        let stride = layout.bytes_per_pixel();
        let alpha_offset = match layout {
            PixelLayout::Rgba8 | PixelLayout::Bgra8 => 3,
            _ => return false,
        };

        pixels
            .chunks_exact(stride)
            .any(|pixel| pixel[alpha_offset] < 255)
    }

    /// Dispatch to format-specific encoder.
    fn encode_format(
        self,
        format: ImageFormat,
        pixels: &[u8],
        width: u32,
        height: u32,
        layout: PixelLayout,
    ) -> Result<EncodeOutput, CodecError> {
        match format {
            #[cfg(feature = "jpeg")]
            ImageFormat::Jpeg => self.encode_jpeg(pixels, width, height, layout),
            #[cfg(not(feature = "jpeg"))]
            ImageFormat::Jpeg => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "webp")]
            ImageFormat::WebP => self.encode_webp(pixels, width, height, layout),
            #[cfg(not(feature = "webp"))]
            ImageFormat::WebP => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "gif")]
            ImageFormat::Gif => self.encode_gif(pixels, width, height, layout),
            #[cfg(not(feature = "gif"))]
            ImageFormat::Gif => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "png")]
            ImageFormat::Png => self.encode_png(pixels, width, height, layout),
            #[cfg(not(feature = "png"))]
            ImageFormat::Png => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "avif-encode")]
            ImageFormat::Avif => self.encode_avif(pixels, width, height, layout),
            #[cfg(not(feature = "avif-encode"))]
            ImageFormat::Avif => Err(CodecError::UnsupportedFormat(format)),
        }
    }

    // Codec-specific encode implementations (stubs for now)

    #[cfg(feature = "jpeg")]
    fn encode_jpeg(
        self,
        _pixels: &[u8],
        _width: u32,
        _height: u32,
        _layout: PixelLayout,
    ) -> Result<EncodeOutput, CodecError> {
        // TODO: implement with zenjpeg adapter
        Err(CodecError::UnsupportedOperation {
            format: ImageFormat::Jpeg,
            detail: "encode not yet implemented",
        })
    }

    #[cfg(feature = "webp")]
    fn encode_webp(
        self,
        _pixels: &[u8],
        _width: u32,
        _height: u32,
        _layout: PixelLayout,
    ) -> Result<EncodeOutput, CodecError> {
        // TODO: implement with zenwebp adapter
        Err(CodecError::UnsupportedOperation {
            format: ImageFormat::WebP,
            detail: "encode not yet implemented",
        })
    }

    #[cfg(feature = "gif")]
    fn encode_gif(
        self,
        _pixels: &[u8],
        _width: u32,
        _height: u32,
        _layout: PixelLayout,
    ) -> Result<EncodeOutput, CodecError> {
        // TODO: implement with zengif adapter
        Err(CodecError::UnsupportedOperation {
            format: ImageFormat::Gif,
            detail: "encode not yet implemented",
        })
    }

    #[cfg(feature = "png")]
    fn encode_png(
        self,
        _pixels: &[u8],
        _width: u32,
        _height: u32,
        _layout: PixelLayout,
    ) -> Result<EncodeOutput, CodecError> {
        // TODO: implement with png adapter
        Err(CodecError::UnsupportedOperation {
            format: ImageFormat::Png,
            detail: "encode not yet implemented",
        })
    }

    #[cfg(feature = "avif-encode")]
    fn encode_avif(
        self,
        _pixels: &[u8],
        _width: u32,
        _height: u32,
        _layout: PixelLayout,
    ) -> Result<EncodeOutput, CodecError> {
        // TODO: implement with ravif adapter
        Err(CodecError::UnsupportedOperation {
            format: ImageFormat::Avif,
            detail: "encode not yet implemented",
        })
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
        assert_eq!(request.lossless, false);
    }

    #[test]
    fn lossless_with_jpeg_error() {
        let pixels = vec![255u8; 100 * 100 * 3];
        let result = EncodeRequest::new(ImageFormat::Jpeg)
            .with_lossless(true)
            .encode(&pixels, 100, 100, PixelLayout::Rgb8);

        assert!(matches!(
            result,
            Err(CodecError::UnsupportedOperation { .. })
        ));
    }

    #[test]
    fn buffer_too_small_error() {
        let pixels = vec![255u8; 100]; // Too small for 100x100
        let result =
            EncodeRequest::new(ImageFormat::Jpeg).encode(&pixels, 100, 100, PixelLayout::Rgb8);

        assert!(matches!(result, Err(CodecError::InvalidInput(_))));
    }
}
