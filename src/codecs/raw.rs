//! RAW/DNG decode adapter -- delegates to zenraw via trait interface.

use alloc::borrow::Cow;

use crate::config::CodecConfig;
use crate::error::Result;
use crate::limits::to_resource_limits;
use crate::{CodecError, DecodeOutput, ImageInfo, Limits, Stop};
use whereat::at;
use zencodec::decode::{Decode, DecodeJob as _, DecoderConfig as _};

/// The ImageFormat for DNG files, re-exported from zenraw.
pub(crate) fn dng_format() -> zencodec::ImageFormat {
    zencodec::ImageFormat::Custom(&zenraw::DNG_FORMAT)
}

/// The ImageFormat for generic RAW files, re-exported from zenraw.
pub(crate) fn raw_format() -> zencodec::ImageFormat {
    zencodec::ImageFormat::Custom(&zenraw::RAW_FORMAT)
}

/// Detect the specific RAW format (DNG vs generic RAW).
pub(crate) fn detect_raw_format(data: &[u8]) -> Option<zencodec::ImageFormat> {
    if !zenraw::is_raw_file(data) {
        return None;
    }
    // zenraw classifies into specific sub-formats
    let classified = zenraw::classify(data);
    match classified {
        zenraw::FileFormat::Dng | zenraw::FileFormat::AppleDng => Some(dng_format()),
        _ if classified.is_raw() => Some(raw_format()),
        _ => None,
    }
}

/// Build a RawDecoderConfig, optionally applying codec config overrides.
fn build_raw_decoder(codec_config: Option<&CodecConfig>) -> zenraw::RawDecoderConfig {
    let mut config = zenraw::RawDecoderConfig::new();
    if let Some(cfg) = codec_config.and_then(|c| c.raw_decoder.as_ref()) {
        config = zenraw::RawDecoderConfig::from_config(cfg.as_ref().clone());
    }
    config
}

/// Map a zenraw error to a CodecError.
fn map_err(format: zencodec::ImageFormat, e: impl core::error::Error + Send + Sync + 'static) -> whereat::At<CodecError> {
    at!(CodecError::Codec {
        format,
        source: alloc::boxed::Box::new(e),
    })
}

/// Probe RAW/DNG metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo> {
    let format = detect_raw_format(data)
        .unwrap_or_else(raw_format);
    let dec = build_raw_decoder(None);
    let job = dec.job();
    job.probe(data).map_err(|e| map_err(format, e))
}

/// Decode RAW/DNG to pixels.
pub(crate) fn decode(
    data: &[u8],
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<DecodeOutput> {
    let format = detect_raw_format(data)
        .unwrap_or_else(raw_format);
    let dec = build_raw_decoder(codec_config);
    let mut job = dec.job();
    if let Some(lim) = limits {
        job = job.with_limits(to_resource_limits(lim));
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.decoder(Cow::Borrowed(data), &[])
        .map_err(|e| map_err(format, e))?
        .decode()
        .map_err(|e| map_err(format, e))
}

/// Extract the embedded JPEG preview from a DNG/TIFF RAW file.
///
/// DNG files commonly contain a reduced-resolution JPEG preview in IFD0.
/// Apple ProRAW files embed a full-resolution sRGB JPEG rendered by the
/// camera pipeline.
///
/// Returns the raw JPEG bytes, or `None` if no preview is found.
#[cfg(feature = "raw-decode-exif")]
pub(crate) fn extract_preview(data: &[u8]) -> Option<alloc::vec::Vec<u8>> {
    zenraw::exif::extract_dng_preview(data)
}

/// Read structured EXIF+DNG metadata from a RAW file using zenraw's
/// kamadak-exif-based parser.
///
/// This returns zenraw's own `ExifMetadata` which includes DNG-specific
/// fields (color matrices, white balance, calibration illuminants).
#[cfg(feature = "raw-decode-exif")]
pub(crate) fn read_raw_metadata(data: &[u8]) -> Option<zenraw::exif::ExifMetadata> {
    zenraw::exif::read_metadata(data)
}
