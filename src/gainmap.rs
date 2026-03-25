//! Format-agnostic gain map types and orchestration.
//!
//! Gain maps enable backward-compatible HDR in image files: a base image
//! (SDR or HDR) plus a secondary gain map image that maps between SDR and HDR
//! renditions. The gain map metadata follows ISO 21496-1, which is used by
//! JPEG (UltraHDR), AVIF (tmap), and JXL (jhgm).
//!
//! # Direction
//!
//! The gain map direction varies by format:
//! - **JPEG/AVIF**: base=SDR, gain map maps SDR→HDR (forward)
//! - **JXL**: base=HDR, gain map maps HDR→SDR (inverse)
//!
//! The [`DecodedGainMap::base_is_hdr`] flag tracks this so callers can
//! determine the correct reconstruction direction.
//!
//! # Gain map image codec
//!
//! The gain map image is encoded with the same codec as the base image
//! (JPEG in JPEG, AV1 in AVIF, JXL in JXL). Decoding and encoding the
//! gain map image is handled internally by the format-specific adapters.
//!
//! # Reconstruction
//!
//! To reconstruct HDR from an SDR base + gain map, use
//! [`ultrahdr_core::apply_gainmap()`] (re-exported via
//! [`zenjpeg::ultrahdr::apply_gainmap`]). That function provides LUT-optimized,
//! streaming-capable reconstruction — far better than reimplementing the math
//! in this crate.

#[cfg(feature = "jpeg-ultrahdr")]
use crate::ImageFormat;

// Re-export the ISO 21496-1 metadata type from ultrahdr-core (via zenjpeg).
#[cfg(feature = "jpeg-ultrahdr")]
pub use zenjpeg::ultrahdr::GainMapMetadata;

// Re-export the gain map pixel type from ultrahdr-core (via zenjpeg).
// This replaces the old `GainMapImage` type that was a duplicate.
#[cfg(feature = "jpeg-ultrahdr")]
pub use zenjpeg::ultrahdr::GainMap;

// Re-export zencodec gain map types.
pub use zencodec::gainmap::{GainMapChannel, GainMapParams, GainMapPresence};

/// Convert [`GainMapParams`] (log2/f64 domain) → [`GainMapMetadata`] (log2/f64 domain).
///
/// Both types now use the same domain, so this is a trivial field copy.
#[cfg(feature = "jpeg-ultrahdr")]
pub fn params_to_metadata(p: &GainMapParams) -> GainMapMetadata {
    GainMapMetadata {
        gain_map_min: [p.channels[0].min, p.channels[1].min, p.channels[2].min],
        gain_map_max: [p.channels[0].max, p.channels[1].max, p.channels[2].max],
        gamma: [
            p.channels[0].gamma,
            p.channels[1].gamma,
            p.channels[2].gamma,
        ],
        base_offset: [
            p.channels[0].base_offset,
            p.channels[1].base_offset,
            p.channels[2].base_offset,
        ],
        alternate_offset: [
            p.channels[0].alternate_offset,
            p.channels[1].alternate_offset,
            p.channels[2].alternate_offset,
        ],
        base_hdr_headroom: p.base_hdr_headroom,
        alternate_hdr_headroom: p.alternate_hdr_headroom,
        use_base_color_space: p.use_base_color_space,
    }
}

/// Convert [`GainMapMetadata`] (log2/f64 domain) → [`GainMapParams`] (log2/f64 domain).
///
/// Both types now use the same domain, so this is a trivial field copy.
#[cfg(feature = "jpeg-ultrahdr")]
pub fn metadata_to_params(m: &GainMapMetadata) -> GainMapParams {
    let mut channels = [GainMapChannel::default(); 3];
    for (i, ch) in channels.iter_mut().enumerate() {
        ch.min = m.gain_map_min[i];
        ch.max = m.gain_map_max[i];
        ch.gamma = m.gamma[i];
        ch.base_offset = m.base_offset[i];
        ch.alternate_offset = m.alternate_offset[i];
    }
    let mut params = GainMapParams::default();
    params.channels = channels;
    params.base_hdr_headroom = m.base_hdr_headroom;
    params.alternate_hdr_headroom = m.alternate_hdr_headroom;
    params.use_base_color_space = m.use_base_color_space;
    params
}

/// Gain map extracted from a decoded image.
///
/// Format-agnostic: works for JPEG (UltraHDR), AVIF (tmap), and JXL (jhgm).
/// The gain map image has already been decoded from the container's embedded
/// format — `gain_map` contains raw pixel data.
///
/// # Reconstruction
///
/// To reconstruct the alternate rendition, use the `gain_map` and `metadata`
/// fields directly with [`zenjpeg::ultrahdr::apply_gainmap()`]:
///
/// ```ignore
/// use zenjpeg::ultrahdr::{apply_gainmap, HdrOutputFormat, Unstoppable};
///
/// let hdr = apply_gainmap(&sdr_image, &decoded.gain_map, &decoded.metadata,
///     display_boost, HdrOutputFormat::LinearFloat, Unstoppable)?;
/// ```
#[derive(Clone, Debug)]
#[cfg(feature = "jpeg-ultrahdr")]
pub struct DecodedGainMap {
    /// The decoded gain map image pixels (grayscale or RGB u8).
    ///
    /// This is the `ultrahdr_core::GainMap` type — pass it directly to
    /// `apply_gainmap()` for HDR reconstruction.
    pub gain_map: GainMap,

    /// ISO 21496-1 gain map metadata describing how to apply the map.
    pub metadata: GainMapMetadata,

    /// Whether the base image is HDR.
    ///
    /// - `false` (JPEG/AVIF): base=SDR, gain map maps SDR→HDR
    /// - `true` (JXL): base=HDR, gain map maps HDR→SDR
    pub base_is_hdr: bool,

    /// Source format this gain map was extracted from.
    pub source_format: ImageFormat,
}

/// Source of gain map data for encoding.
///
/// When encoding an image with a gain map, you can either provide a
/// pre-computed gain map (for passthrough/transcode) or have the encoder
/// compute one from HDR source pixels.
#[cfg(feature = "jpeg-ultrahdr")]
pub enum GainMapSource<'a> {
    /// Pre-computed gain map (for passthrough/transcode).
    ///
    /// The encoder embeds this directly without recomputation. Useful when
    /// transcoding between formats or re-encoding with edits that don't
    /// affect the HDR mapping.
    Precomputed {
        /// The gain map image pixels.
        gain_map: &'a GainMap,
        /// ISO 21496-1 metadata describing the mapping.
        metadata: &'a GainMapMetadata,
    },
}

#[cfg(feature = "jpeg-ultrahdr")]
impl DecodedGainMap {
    /// Convert the stored [`GainMapMetadata`] to the canonical [`GainMapParams`].
    pub fn params(&self) -> GainMapParams {
        metadata_to_params(&self.metadata)
    }

    /// Build a [`GainMapInfo`](zencodec::GainMapInfo) describing this gain map
    /// (metadata + dimensions, no pixel data).
    pub fn to_gain_map_info(&self) -> zencodec::GainMapInfo {
        zencodec::GainMapInfo::new(
            self.params(),
            self.gain_map.width,
            self.gain_map.height,
            self.gain_map.channels,
        )
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[cfg(feature = "jpeg-ultrahdr")]
    #[test]
    fn decoded_gainmap_jpeg_sdr_base() {
        let gm = DecodedGainMap {
            gain_map: GainMap {
                data: alloc::vec![128; 4],
                width: 2,
                height: 2,
                channels: 1,
            },
            metadata: GainMapMetadata {
                gain_map_max: [2.0; 3],
                gain_map_min: [0.0; 3],
                gamma: [1.0; 3],
                base_offset: [1.0 / 64.0; 3],
                alternate_offset: [1.0 / 64.0; 3],
                base_hdr_headroom: 0.0,
                alternate_hdr_headroom: 2.0,
                use_base_color_space: true,
                ..GainMapMetadata::default()
            },
            base_is_hdr: false,
            source_format: ImageFormat::Jpeg,
        };
        assert!(!gm.base_is_hdr);
        assert_eq!(gm.source_format, ImageFormat::Jpeg);
    }

    #[cfg(feature = "jpeg-ultrahdr")]
    #[test]
    fn decoded_gainmap_jxl_hdr_base() {
        let gm = DecodedGainMap {
            gain_map: GainMap {
                data: alloc::vec![128; 4],
                width: 2,
                height: 2,
                channels: 1,
            },
            metadata: GainMapMetadata::default(),
            base_is_hdr: true,
            source_format: ImageFormat::Jxl,
        };
        assert!(gm.base_is_hdr);
        assert_eq!(gm.source_format, ImageFormat::Jxl);
    }

    #[cfg(feature = "jpeg-ultrahdr")]
    #[test]
    fn gainmap_source_precomputed() {
        let img = GainMap {
            data: alloc::vec![200; 8 * 8],
            width: 8,
            height: 8,
            channels: 1,
        };
        let meta = GainMapMetadata {
            gain_map_max: [2.0; 3],
            gain_map_min: [0.0; 3],
            gamma: [1.0; 3],
            base_offset: [1.0 / 64.0; 3],
            alternate_offset: [1.0 / 64.0; 3],
            base_hdr_headroom: 0.0,
            alternate_hdr_headroom: 2.0,
            use_base_color_space: true,
            ..GainMapMetadata::default()
        };
        let source = GainMapSource::Precomputed {
            gain_map: &img,
            metadata: &meta,
        };
        match source {
            GainMapSource::Precomputed { gain_map, metadata } => {
                assert_eq!(gain_map.width, 8);
                assert_eq!(gain_map.height, 8);
                assert_eq!(gain_map.channels, 1);
                assert_eq!(metadata.gain_map_max[0], 2.0);
            }
        }
    }

    #[cfg(feature = "jpeg-ultrahdr")]
    #[test]
    fn params_metadata_roundtrip() {
        let meta = GainMapMetadata {
            gain_map_max: [2.0; 3],
            gain_map_min: [0.0; 3],
            gamma: [1.0; 3],
            base_offset: [1.0 / 64.0; 3],
            alternate_offset: [1.0 / 64.0; 3],
            base_hdr_headroom: 0.0,
            alternate_hdr_headroom: 2.0,
            use_base_color_space: true,
            ..GainMapMetadata::default()
        };
        let params = metadata_to_params(&meta);
        let meta2 = params_to_metadata(&params);
        for i in 0..3 {
            assert!((meta.gain_map_max[i] - meta2.gain_map_max[i]).abs() < 0.01);
            assert!((meta.gain_map_min[i] - meta2.gain_map_min[i]).abs() < 0.01);
            assert!((meta.gamma[i] - meta2.gamma[i]).abs() < 0.01);
        }
    }
}
