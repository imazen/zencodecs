//! AVIF decode adapter using zenavif via trait interface.

use crate::config::CodecConfig;
use crate::limits::to_resource_limits;
use crate::{
    CodecError, DecodeOutput, Decoding, DecodingJob, ImageFormat, ImageInfo, Limits, Stop,
};

/// Probe AVIF metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    zenavif::AvifDecoding::new()
        .probe_header(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))
}

/// Decode AVIF to pixels.
pub(crate) fn decode(
    data: &[u8],
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let mut dec = zenavif::AvifDecoding::new();
    // Apply codec config if provided
    if let Some(cfg) = codec_config.and_then(|c| c.avif_decoder.as_ref()) {
        *dec.inner_mut() = cfg.as_ref().clone();
    }
    if let Some(lim) = limits {
        dec = dec.with_limits(to_resource_limits(lim));
    }
    let mut job = dec.job();
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.decode(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::Avif, e))
}
