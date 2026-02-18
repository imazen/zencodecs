//! JXL decode adapter — delegates to zenjxl via trait interface.

use crate::limits::to_resource_limits;
use crate::{
    CodecError, DecodeJob, DecodeOutput, DecoderConfig, ImageFormat, ImageInfo, Limits, Stop,
};
use zencodec_types::Decoder;

/// Probe JXL metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    zenjxl::JxlDecoderConfig::new()
        .probe_header(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))
}

/// Decode JXL to pixels.
pub(crate) fn decode(
    data: &[u8],
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    // JxlDecoderConfig has inherent with_limits
    let mut dec = zenjxl::JxlDecoderConfig::new();
    if let Some(lim) = limits {
        dec = dec.with_limits(to_resource_limits(lim));
    }
    let mut job = dec.job();
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.decoder()
        .decode(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jxl, e))
}
