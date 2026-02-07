//! AVIF decode adapter using zenavif.
//!
//! TODO: Implement AVIF decode adapter

use crate::{CodecError, DecodeOutput, ImageFormat, ImageInfo, Limits, PixelLayout, Stop};

pub(crate) fn probe(_data: &[u8]) -> Result<ImageInfo, CodecError> {
    Err(CodecError::UnsupportedOperation {
        format: ImageFormat::Avif,
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
        format: ImageFormat::Avif,
        detail: "decode not yet implemented",
    })
}
