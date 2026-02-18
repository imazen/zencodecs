//! Pixel format helpers for the pipeline.
//!
//! Most conversion is handled by `PixelData::into_rgb8()`, `into_rgba8()`,
//! and `into_gray8()` on `DecodeOutput`. This module provides additional
//! helpers for pipeline-specific decisions.

use zencodec_types::PixelData;

/// Whether the pixel data needs an alpha channel for encoding.
///
/// Returns `true` if the source has alpha AND any pixel is non-opaque.
/// For formats without alpha (like JPEG), the pipeline can drop to RGB.
pub(crate) fn needs_alpha(pixels: &PixelData) -> bool {
    pixels.has_alpha()
}

/// Whether the pixel data is grayscale (no color channels).
pub(crate) fn is_grayscale(pixels: &PixelData) -> bool {
    pixels.is_grayscale()
}
