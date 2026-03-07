//! Dynamic encoder dispatch.
//!
//! Provides [`build_encoder`] factory that creates a type-erased encoder closure
//! for any supported format. Each codec's `Encoder` trait impl handles pixel
//! format dispatch internally.

use crate::config::CodecConfig;
use crate::{CodecError, ImageFormat, Limits, MetadataView, Stop};
use alloc::boxed::Box;
use zencodec_types::EncodeOutput;
use zenpixels::{PixelDescriptor, PixelSlice};

/// Encoding parameters extracted from [`EncodeRequest`](crate::EncodeRequest).
pub(crate) struct EncodeParams<'a> {
    pub quality: Option<f32>,
    pub effort: Option<u32>,
    pub lossless: bool,
    pub metadata: Option<&'a MetadataView<'a>>,
    pub codec_config: Option<&'a CodecConfig>,
    pub limits: Option<&'a Limits>,
    pub stop: Option<&'a dyn Stop>,
}

/// Type-erased one-shot encode closure.
pub(crate) type EncodeFn<'a> =
    Box<dyn FnOnce(PixelSlice<'_>) -> Result<EncodeOutput, CodecError> + 'a>;

/// A built encoder: a closure that encodes pixels + its supported descriptors.
pub(crate) struct BuiltEncoder<'a> {
    pub encoder: EncodeFn<'a>,
    pub supported: &'static [PixelDescriptor],
}

/// Build a type-erased encoder from a config-building closure.
///
/// The closure receives `EncodeParams` and returns the concrete `EncoderConfig`.
/// Config construction happens inside the returned closure so the config's
/// lifetime doesn't escape the function.
pub(crate) fn build_from_config<'a, C, F>(build_config: F, params: EncodeParams<'a>) -> BuiltEncoder<'a>
where
    C: zencodec_types::EncoderConfig + 'a,
    F: FnOnce(&EncodeParams<'a>) -> C + 'a,
    for<'b> <C::Job<'b> as zencodec_types::EncodeJob<'b>>::Enc: zencodec_types::Encoder,
{
    BuiltEncoder {
        encoder: Box::new(move |pixels| {
            use zencodec_types::EncodeJob as _;
            let config = build_config(&params);
            let mut job = config.job();
            if let Some(lim) = params.limits {
                job = job.with_limits(crate::limits::to_resource_limits(lim));
            }
            if let Some(meta) = params.metadata {
                job = job.with_metadata(meta);
            }
            if let Some(s) = params.stop {
                job = job.with_stop(s);
            }
            let format = C::format();
            job.dyn_encoder()
                .map_err(|e| CodecError::from_codec_boxed(format, e))?
                .encode(pixels)
                .map_err(|e| CodecError::from_codec_boxed(format, e))
        }),
        supported: C::supported_descriptors(),
    }
}

/// Build a type-erased encoder for the specified format.
///
/// Each codec arm delegates to its `build_trait_encoder` which builds
/// the codec-specific config, creates the encode job, and returns
/// a closure that calls `Encoder::encode(pixels)` via the trait.
pub(crate) fn build_encoder<'a>(
    format: ImageFormat,
    params: EncodeParams<'a>,
) -> Result<BuiltEncoder<'a>, CodecError> {
    match format {
        #[cfg(feature = "jpeg")]
        ImageFormat::Jpeg => Ok(crate::codecs::jpeg::build_trait_encoder(params)),
        #[cfg(not(feature = "jpeg"))]
        ImageFormat::Jpeg => Err(CodecError::UnsupportedFormat(format)),

        #[cfg(feature = "webp")]
        ImageFormat::WebP => Ok(crate::codecs::webp::build_trait_encoder(params)),
        #[cfg(not(feature = "webp"))]
        ImageFormat::WebP => Err(CodecError::UnsupportedFormat(format)),

        #[cfg(feature = "gif")]
        ImageFormat::Gif => Ok(crate::codecs::gif::build_trait_encoder(params)),
        #[cfg(not(feature = "gif"))]
        ImageFormat::Gif => Err(CodecError::UnsupportedFormat(format)),

        #[cfg(feature = "png")]
        ImageFormat::Png => Ok(crate::codecs::png::build_trait_encoder(params)),
        #[cfg(not(feature = "png"))]
        ImageFormat::Png => Err(CodecError::UnsupportedFormat(format)),

        #[cfg(feature = "avif-encode")]
        ImageFormat::Avif => Ok(crate::codecs::avif_enc::build_trait_encoder(params)),
        #[cfg(not(feature = "avif-encode"))]
        ImageFormat::Avif => Err(CodecError::UnsupportedFormat(format)),

        #[cfg(feature = "jxl-encode")]
        ImageFormat::Jxl => Ok(crate::codecs::jxl_enc::build_trait_encoder(params)),
        #[cfg(not(feature = "jxl-encode"))]
        ImageFormat::Jxl => Err(CodecError::UnsupportedFormat(format)),

        _ => Err(CodecError::UnsupportedFormat(format)),
    }
}
