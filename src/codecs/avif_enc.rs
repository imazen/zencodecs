//! AVIF encode adapter using ravif.
//!
//! TODO: Implement AVIF encode adapter

use crate::{CodecError, EncodeOutput, ImageFormat, Limits, PixelLayout, Stop};

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
        format: ImageFormat::Avif,
        detail: "encode not yet implemented",
    })
}
