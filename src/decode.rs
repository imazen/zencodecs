//! Image decoding.

use alloc::vec::Vec;

use crate::{CodecError, CodecRegistry, ImageFormat, ImageInfo, Limits, PixelLayout, Stop};

/// Decoded image output.
#[derive(Clone, Debug)]
pub struct DecodeOutput {
    /// Decoded pixel data in the requested layout.
    pub pixels: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// Pixel layout of the decoded data.
    pub layout: PixelLayout,
    /// Image metadata.
    pub info: ImageInfo,
}

/// Image decode request builder.
///
/// Use this to decode images from various formats to a unified pixel layout.
///
/// # Example
///
/// ```no_run
/// use zencodecs::{DecodeRequest, PixelLayout};
///
/// let data: &[u8] = &[]; // your image bytes
/// let output = DecodeRequest::new(data)
///     .with_output_layout(PixelLayout::Rgba8)
///     .decode()?;
/// # Ok::<(), zencodecs::CodecError>(())
/// ```
pub struct DecodeRequest<'a> {
    data: &'a [u8],
    format: Option<ImageFormat>,
    output_layout: PixelLayout,
    limits: Option<&'a Limits>,
    stop: Option<&'a dyn Stop>,
    registry: Option<&'a CodecRegistry>,
}

impl<'a> DecodeRequest<'a> {
    /// Create a new decode request.
    ///
    /// Format will be auto-detected from magic bytes.
    /// Default output layout is RGBA8.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            format: None,
            output_layout: PixelLayout::default(),
            limits: None,
            stop: None,
            registry: None,
        }
    }

    /// Override format auto-detection.
    ///
    /// Use this when you already know the format and want to skip detection.
    pub fn with_format(mut self, format: ImageFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Set the output pixel layout.
    ///
    /// The decoder will convert to this layout regardless of the source format.
    pub fn with_output_layout(mut self, layout: PixelLayout) -> Self {
        self.output_layout = layout;
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

    /// Set a codec registry to control which formats are enabled.
    ///
    /// If not set, all compiled-in codecs are available.
    pub fn with_registry(mut self, registry: &'a CodecRegistry) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Decode the image to pixels.
    pub fn decode(self) -> Result<DecodeOutput, CodecError> {
        // Get the registry (default to all if not specified)
        let default_registry = CodecRegistry::all();
        let registry = self.registry.unwrap_or(&default_registry);

        // Detect format if not specified
        let format = match self.format {
            Some(f) => f,
            None => ImageFormat::detect(self.data).ok_or(CodecError::UnrecognizedFormat)?,
        };

        // Check if format is enabled
        if !registry.can_decode(format) {
            return Err(CodecError::DisabledFormat(format));
        }

        // Dispatch to format-specific decoder
        self.decode_format(format)
    }

    /// Decode into a pre-allocated buffer.
    ///
    /// The buffer must be large enough for the output (width × height × bpp).
    /// Returns the image info on success.
    pub fn decode_into(self, output: &mut [u8]) -> Result<ImageInfo, CodecError> {
        let decoded = self.decode()?;

        if output.len() < decoded.pixels.len() {
            return Err(CodecError::InvalidInput("output buffer too small".into()));
        }

        output[..decoded.pixels.len()].copy_from_slice(&decoded.pixels);
        Ok(decoded.info)
    }

    /// Dispatch to format-specific decoder.
    fn decode_format(self, format: ImageFormat) -> Result<DecodeOutput, CodecError> {
        match format {
            #[cfg(feature = "jpeg")]
            ImageFormat::Jpeg => self.decode_jpeg(),
            #[cfg(not(feature = "jpeg"))]
            ImageFormat::Jpeg => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "webp")]
            ImageFormat::WebP => self.decode_webp(),
            #[cfg(not(feature = "webp"))]
            ImageFormat::WebP => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "gif")]
            ImageFormat::Gif => self.decode_gif(),
            #[cfg(not(feature = "gif"))]
            ImageFormat::Gif => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "png")]
            ImageFormat::Png => self.decode_png(),
            #[cfg(not(feature = "png"))]
            ImageFormat::Png => Err(CodecError::UnsupportedFormat(format)),

            #[cfg(feature = "avif-decode")]
            ImageFormat::Avif => self.decode_avif(),
            #[cfg(not(feature = "avif-decode"))]
            ImageFormat::Avif => Err(CodecError::UnsupportedFormat(format)),
        }
    }

    // Codec-specific decode implementations (stubs for now)

    #[cfg(feature = "jpeg")]
    fn decode_jpeg(self) -> Result<DecodeOutput, CodecError> {
        crate::codecs::jpeg::decode(self.data, self.output_layout, self.limits, self.stop)
    }

    #[cfg(feature = "webp")]
    fn decode_webp(self) -> Result<DecodeOutput, CodecError> {
        crate::codecs::webp::decode(self.data, self.output_layout, self.limits, self.stop)
    }

    #[cfg(feature = "gif")]
    fn decode_gif(self) -> Result<DecodeOutput, CodecError> {
        crate::codecs::gif::decode(self.data, self.output_layout, self.limits, self.stop)
    }

    #[cfg(feature = "png")]
    fn decode_png(self) -> Result<DecodeOutput, CodecError> {
        crate::codecs::png::decode(self.data, self.output_layout, self.limits, self.stop)
    }

    #[cfg(feature = "avif-decode")]
    fn decode_avif(self) -> Result<DecodeOutput, CodecError> {
        crate::codecs::avif_dec::decode(self.data, self.output_layout, self.limits, self.stop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_pattern() {
        let data = b"test";
        let request = DecodeRequest::new(data)
            .with_format(ImageFormat::Jpeg)
            .with_output_layout(PixelLayout::Rgb8);

        assert_eq!(request.format, Some(ImageFormat::Jpeg));
        assert_eq!(request.output_layout, PixelLayout::Rgb8);
    }

    #[test]
    fn disabled_format_error() {
        let jpeg_data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        let registry = CodecRegistry::none();

        let result = DecodeRequest::new(&jpeg_data)
            .with_registry(&registry)
            .decode();

        assert!(matches!(result, Err(CodecError::DisabledFormat(_))));
    }
}
