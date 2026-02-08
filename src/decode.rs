//! Image decoding.

use crate::config::CodecConfig;
use crate::{CodecError, CodecRegistry, ImageFormat, ImageInfo, Limits, PixelData, Stop};

/// Decoded image output.
#[derive(Debug)]
pub struct DecodeOutput {
    /// Decoded pixel data in a typed buffer.
    pub pixels: PixelData,
    /// Image metadata.
    pub info: ImageInfo,
}

impl DecodeOutput {
    /// Image width in pixels (convenience accessor).
    pub fn width(&self) -> u32 {
        self.pixels.width()
    }

    /// Image height in pixels (convenience accessor).
    pub fn height(&self) -> u32 {
        self.pixels.height()
    }
}

/// Image decode request builder.
///
/// # Example
///
/// ```no_run
/// use zencodecs::DecodeRequest;
///
/// let data: &[u8] = &[]; // your image bytes
/// let output = DecodeRequest::new(data).decode()?;
/// println!("{}x{}", output.width(), output.height());
/// # Ok::<(), zencodecs::CodecError>(())
/// ```
pub struct DecodeRequest<'a> {
    data: &'a [u8],
    format: Option<ImageFormat>,
    limits: Option<&'a Limits>,
    stop: Option<&'a dyn Stop>,
    registry: Option<&'a CodecRegistry>,
    codec_config: Option<&'a CodecConfig>,
}

impl<'a> DecodeRequest<'a> {
    /// Create a new decode request.
    ///
    /// Format will be auto-detected from magic bytes.
    /// The decoder returns its native pixel format.
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            format: None,
            limits: None,
            stop: None,
            registry: None,
            codec_config: None,
        }
    }

    /// Override format auto-detection.
    pub fn with_format(mut self, format: ImageFormat) -> Self {
        self.format = Some(format);
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
    pub fn with_registry(mut self, registry: &'a CodecRegistry) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Set format-specific codec configuration.
    pub fn with_codec_config(mut self, config: &'a CodecConfig) -> Self {
        self.codec_config = Some(config);
        self
    }

    /// Decode the image to pixels.
    pub fn decode(self) -> Result<DecodeOutput, CodecError> {
        let default_registry = CodecRegistry::all();
        let registry = self.registry.unwrap_or(&default_registry);

        let format = match self.format {
            Some(f) => f,
            None => ImageFormat::detect(self.data).ok_or(CodecError::UnrecognizedFormat)?,
        };

        if !registry.can_decode(format) {
            return Err(CodecError::DisabledFormat(format));
        }

        self.decode_format(format)
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

    #[cfg(feature = "jpeg")]
    fn decode_jpeg(self) -> Result<DecodeOutput, CodecError> {
        crate::codecs::jpeg::decode(self.data, self.codec_config, self.limits, self.stop)
    }

    #[cfg(feature = "webp")]
    fn decode_webp(self) -> Result<DecodeOutput, CodecError> {
        crate::codecs::webp::decode(self.data, self.limits, self.stop)
    }

    #[cfg(feature = "gif")]
    fn decode_gif(self) -> Result<DecodeOutput, CodecError> {
        crate::codecs::gif::decode(self.data, self.limits, self.stop)
    }

    #[cfg(feature = "png")]
    fn decode_png(self) -> Result<DecodeOutput, CodecError> {
        crate::codecs::png::decode(self.data, self.limits, self.stop)
    }

    #[cfg(feature = "avif-decode")]
    fn decode_avif(self) -> Result<DecodeOutput, CodecError> {
        crate::codecs::avif_dec::decode(self.data, self.codec_config, self.limits, self.stop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_pattern() {
        let data = b"test";
        let request = DecodeRequest::new(data).with_format(ImageFormat::Jpeg);
        assert_eq!(request.format, Some(ImageFormat::Jpeg));
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
