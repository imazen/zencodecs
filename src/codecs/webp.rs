//! WebP codec adapter using zenwebp.
//!
//! TODO: Implement WebP codec adapter

use crate::{
    CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, Limits, PixelLayout, Stop,
};

pub(crate) fn probe(_data: &[u8]) -> Result<ImageInfo, CodecError> {
    Err(CodecError::UnsupportedOperation {
        format: ImageFormat::WebP,
        detail: "probe not yet implemented",
    })
}

pub(crate) fn decode(
    _data: &[u8],
    _output_layout: PixelLayout,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    Err(CodecError::UnsupportedOperation {
        format: ImageFormat::WebP,
        detail: "decode not yet implemented",
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode(
    _pixels: &[u8],
    _width: u32,
    _height: u32,
    _layout: PixelLayout,
    _quality: Option<f32>,
    _lossless: bool,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    Err(CodecError::UnsupportedOperation {
        format: ImageFormat::WebP,
        detail: "encode not yet implemented",
    })
}
