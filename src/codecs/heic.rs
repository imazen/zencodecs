//! HEIC decode adapter -- delegates to heic-decoder via trait interface.

use alloc::borrow::Cow;

use crate::error::Result;
use crate::limits::to_resource_limits;
use crate::{
    CodecError, DecodeJob, DecodeOutput, DecoderConfig, ImageFormat, ImageInfo, Limits, StopToken,
};
use whereat::at;
use zencodec::decode::Decode;

/// Probe HEIC metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo> {
    let dec = heic_decoder::HeicDecoderConfig::new();
    let job = dec.job();
    job.probe(data)
        .map_err(|e| at!(CodecError::from_codec(ImageFormat::Heic, e)))
}

/// Decode HEIC to pixels.
pub(crate) fn decode(
    data: &[u8],
    limits: Option<&Limits>,
    stop: Option<StopToken>,
    decode_policy: Option<zencodec::decode::DecodePolicy>,
) -> Result<DecodeOutput> {
    let dec = heic_decoder::HeicDecoderConfig::new();
    let mut job = dec.job();
    if let Some(lim) = limits {
        job = job.with_limits(to_resource_limits(lim));
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    if let Some(dp) = decode_policy {
        job = job.with_policy(dp);
    }
    job.decoder(Cow::Borrowed(data), &[])
        .map_err(|e| at!(CodecError::from_codec(ImageFormat::Heic, e)))?
        .decode()
        .map_err(|e| at!(CodecError::from_codec(ImageFormat::Heic, e)))
}
