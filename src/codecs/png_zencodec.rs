//! zencodec-types trait implementations for PNG.
//!
//! Provides [`PngEncoding`] and [`PngDecoding`] types that implement the
//! [`Encoding`] / [`Decoding`] traits from zencodec-types, wrapping the
//! PNG adapter in `codecs::png`.

extern crate std;

use alloc::vec::Vec;

use imgref::ImgRef;
use rgb::{Gray, Rgb, Rgba};

use zencodec_types::{
    DecodeOutput as ZDecodeOutput, Decoding, DecodingJob, EncodeOutput as ZEncodeOutput, Encoding,
    EncodingJob, ImageFormat as ZImageFormat, ImageInfo as ZImageInfo,
    ImageMetadata as ZImageMetadata, PixelData as ZPixelData, Stop,
};

use crate::error::CodecError;

// ── Encoding ────────────────────────────────────────────────────────────────

/// PNG encoder configuration implementing [`Encoding`].
///
/// PNG is lossless — quality and alpha_quality have no effect.
/// Effort maps to png compression level.
///
/// # Examples
///
/// ```rust,ignore
/// use zencodec_types::Encoding;
/// use zencodecs::PngEncoding;
///
/// let enc = PngEncoding::new()
///     .with_effort(5); // default compression
/// ```
#[derive(Clone, Debug)]
pub struct PngEncoding {
    compression: png::Compression,
    filter: png::Filter,
    limit_pixels: Option<u64>,
    limit_memory: Option<u64>,
    limit_output: Option<u64>,
}

impl PngEncoding {
    /// Create a default PNG encoder config.
    #[must_use]
    pub fn new() -> Self {
        Self {
            compression: png::Compression::default(),
            filter: png::Filter::default(),
            limit_pixels: None,
            limit_memory: None,
            limit_output: None,
        }
    }

    /// Set PNG compression level directly.
    #[must_use]
    pub fn with_compression(mut self, compression: png::Compression) -> Self {
        self.compression = compression;
        self
    }

    /// Set PNG row filter type directly.
    #[must_use]
    pub fn with_filter(mut self, filter: png::Filter) -> Self {
        self.filter = filter;
        self
    }
}

impl Default for PngEncoding {
    fn default() -> Self {
        Self::new()
    }
}

impl Encoding for PngEncoding {
    type Error = CodecError;
    type Job<'a> = PngEncodeJob<'a>;

    fn with_quality(self, _quality: f32) -> Self {
        // PNG is lossless; quality has no effect.
        self
    }

    fn with_effort(mut self, effort: u32) -> Self {
        // Map 0-10 effort to png::Compression.
        self.compression = match effort {
            0..=2 => png::Compression::Fast,
            3..=7 => png::Compression::Balanced,
            _ => png::Compression::High,
        };
        self
    }

    fn with_lossless(self, _lossless: bool) -> Self {
        // PNG is always lossless; ignore.
        self
    }

    fn with_alpha_quality(self, _quality: f32) -> Self {
        // PNG doesn't have separate alpha quality.
        self
    }

    fn with_limit_pixels(mut self, max: u64) -> Self {
        self.limit_pixels = Some(max);
        self
    }

    fn with_limit_memory(mut self, bytes: u64) -> Self {
        self.limit_memory = Some(bytes);
        self
    }

    fn with_limit_output(mut self, bytes: u64) -> Self {
        self.limit_output = Some(bytes);
        self
    }

    fn job(&self) -> PngEncodeJob<'_> {
        PngEncodeJob {
            config: self,
            icc: None,
            exif: None,
            xmp: None,
            limit_pixels: None,
            limit_memory: None,
        }
    }
}

// ── Encode job ──────────────────────────────────────────────────────────────

/// Per-operation PNG encode job.
pub struct PngEncodeJob<'a> {
    config: &'a PngEncoding,
    icc: Option<&'a [u8]>,
    exif: Option<&'a [u8]>,
    xmp: Option<&'a [u8]>,
    limit_pixels: Option<u64>,
    limit_memory: Option<u64>,
}

impl<'a> PngEncodeJob<'a> {
    /// Build metadata for the PNG encoder.
    fn build_metadata(&self) -> Option<crate::limits::ImageMetadata<'a>> {
        if self.icc.is_none() && self.exif.is_none() && self.xmp.is_none() {
            return None;
        }
        Some(crate::limits::ImageMetadata {
            icc_profile: self.icc,
            exif: self.exif,
            xmp: self.xmp,
        })
    }

    /// Common encode path.
    fn do_encode(
        self,
        bytes: &[u8],
        w: u32,
        h: u32,
        color_type: png::ColorType,
    ) -> Result<ZEncodeOutput, CodecError> {
        let mut output = Vec::new();

        let meta = self.build_metadata();
        let zencodecs_meta: Option<crate::limits::ImageMetadata<'_>> = meta;

        let mut info = png::Info::with_size(w, h);
        info.color_type = color_type;
        info.bit_depth = png::BitDepth::Eight;

        if let Some(ref m) = zencodecs_meta {
            if let Some(icc) = m.icc_profile {
                info.icc_profile = Some(icc.into());
            }
            if let Some(exif) = m.exif {
                info.exif_metadata = Some(exif.into());
            }
            if let Some(xmp) = m.xmp {
                let xmp_str = core::str::from_utf8(xmp).unwrap_or_default();
                if !xmp_str.is_empty() {
                    info.utf8_text.push(png::text_metadata::ITXtChunk::new(
                        "XML:com.adobe.xmp",
                        xmp_str,
                    ));
                }
            }
        }

        let mut encoder = png::Encoder::with_info(&mut output, info)
            .map_err(|e| CodecError::from_codec(crate::ImageFormat::Png, e))?;
        encoder.set_compression(self.config.compression);
        encoder.set_filter(self.config.filter);

        let mut writer = encoder
            .write_header()
            .map_err(|e| CodecError::from_codec(crate::ImageFormat::Png, e))?;

        writer
            .write_image_data(bytes)
            .map_err(|e| CodecError::from_codec(crate::ImageFormat::Png, e))?;

        drop(writer);

        Ok(ZEncodeOutput::new(output, ZImageFormat::Png))
    }
}

impl<'a> EncodingJob<'a> for PngEncodeJob<'a> {
    type Error = CodecError;

    fn with_stop(self, _stop: &'a dyn Stop) -> Self {
        // PNG encoding is not cancellable in the png crate.
        self
    }

    fn with_metadata(mut self, meta: &'a ZImageMetadata<'a>) -> Self {
        if let Some(icc) = meta.icc_profile {
            self.icc = Some(icc);
        }
        if let Some(exif) = meta.exif {
            self.exif = Some(exif);
        }
        if let Some(xmp) = meta.xmp {
            self.xmp = Some(xmp);
        }
        self
    }

    fn with_icc(mut self, icc: &'a [u8]) -> Self {
        self.icc = Some(icc);
        self
    }

    fn with_exif(mut self, exif: &'a [u8]) -> Self {
        self.exif = Some(exif);
        self
    }

    fn with_xmp(mut self, xmp: &'a [u8]) -> Self {
        self.xmp = Some(xmp);
        self
    }

    fn with_limit_pixels(mut self, max: u64) -> Self {
        self.limit_pixels = Some(max);
        self
    }

    fn with_limit_memory(mut self, bytes: u64) -> Self {
        self.limit_memory = Some(bytes);
        self
    }

    fn encode_rgb8(self, img: ImgRef<'_, Rgb<u8>>) -> Result<ZEncodeOutput, Self::Error> {
        let (buf, w, h) = img.to_contiguous_buf();
        let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());
        self.do_encode(bytes, w as u32, h as u32, png::ColorType::Rgb)
    }

    fn encode_rgba8(self, img: ImgRef<'_, Rgba<u8>>) -> Result<ZEncodeOutput, Self::Error> {
        let (buf, w, h) = img.to_contiguous_buf();
        let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());
        self.do_encode(bytes, w as u32, h as u32, png::ColorType::Rgba)
    }

    fn encode_gray8(self, img: ImgRef<'_, Gray<u8>>) -> Result<ZEncodeOutput, Self::Error> {
        let (buf, w, h) = img.to_contiguous_buf();
        let bytes: Vec<u8> = buf.iter().map(|g| g.value()).collect();
        self.do_encode(&bytes, w as u32, h as u32, png::ColorType::Grayscale)
    }
}

// ── Decoding ────────────────────────────────────────────────────────────────

/// PNG decoder configuration implementing [`Decoding`].
#[derive(Clone, Debug)]
pub struct PngDecoding {
    limit_pixels: Option<u64>,
    limit_memory: Option<u64>,
    limit_file_size: Option<u64>,
}

impl PngDecoding {
    /// Create a default PNG decoder config.
    #[must_use]
    pub fn new() -> Self {
        Self {
            limit_pixels: None,
            limit_memory: None,
            limit_file_size: None,
        }
    }
}

impl Default for PngDecoding {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoding for PngDecoding {
    type Error = CodecError;
    type Job<'a> = PngDecodeJob<'a>;

    fn with_limit_pixels(mut self, max: u64) -> Self {
        self.limit_pixels = Some(max);
        self
    }

    fn with_limit_memory(mut self, bytes: u64) -> Self {
        self.limit_memory = Some(bytes);
        self
    }

    fn with_limit_dimensions(mut self, width: u32, height: u32) -> Self {
        self.limit_pixels = Some(width as u64 * height as u64);
        self
    }

    fn with_limit_file_size(mut self, bytes: u64) -> Self {
        self.limit_file_size = Some(bytes);
        self
    }

    fn job(&self) -> PngDecodeJob<'_> {
        PngDecodeJob {
            config: self,
            limit_pixels: None,
            limit_memory: None,
        }
    }

    fn probe(&self, data: &[u8]) -> Result<ZImageInfo, Self::Error> {
        let internal_info = super::png::probe(data)?;
        Ok(convert_info(&internal_info))
    }
}

// ── Decode job ──────────────────────────────────────────────────────────────

/// Per-operation PNG decode job.
pub struct PngDecodeJob<'a> {
    config: &'a PngDecoding,
    limit_pixels: Option<u64>,
    limit_memory: Option<u64>,
}

impl<'a> DecodingJob<'a> for PngDecodeJob<'a> {
    type Error = CodecError;

    fn with_stop(self, _stop: &'a dyn Stop) -> Self {
        // PNG decoding is not cancellable in the png crate.
        self
    }

    fn with_limit_pixels(mut self, max: u64) -> Self {
        self.limit_pixels = Some(max);
        self
    }

    fn with_limit_memory(mut self, bytes: u64) -> Self {
        self.limit_memory = Some(bytes);
        self
    }

    fn decode(self, data: &[u8]) -> Result<ZDecodeOutput, Self::Error> {
        // Build limits
        let limits = if self.limit_pixels.is_some()
            || self.limit_memory.is_some()
            || self.config.limit_pixels.is_some()
            || self.config.limit_memory.is_some()
        {
            let mut l = crate::Limits::default();
            if let Some(p) = self.limit_pixels.or(self.config.limit_pixels) {
                l.max_pixels = Some(p);
            }
            if let Some(m) = self.limit_memory.or(self.config.limit_memory) {
                l.max_memory_bytes = Some(m);
            }
            Some(l)
        } else {
            None
        };

        // Use existing PNG decode
        let result = super::png::decode(data, limits.as_ref(), None)?;

        // Convert zencodecs types to zencodec-types
        let info = convert_info(&result.info);
        let pixels = convert_pixels(result.pixels);

        Ok(ZDecodeOutput::new(pixels, info))
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Convert zencodecs::ImageInfo to zencodec_types::ImageInfo.
fn convert_info(info: &crate::ImageInfo) -> ZImageInfo {
    let mut zi = ZImageInfo::new(info.width, info.height, ZImageFormat::Png);
    if info.has_alpha {
        zi = zi.with_alpha(true);
    }
    if info.has_animation {
        zi = zi.with_animation(true);
    }
    if let Some(count) = info.frame_count {
        zi = zi.with_frame_count(count);
    }
    if let Some(ref icc) = info.icc_profile {
        zi = zi.with_icc_profile(icc.clone());
    }
    if let Some(ref exif) = info.exif {
        zi = zi.with_exif(exif.clone());
    }
    if let Some(ref xmp) = info.xmp {
        zi = zi.with_xmp(xmp.clone());
    }
    zi
}

/// Convert zencodecs::PixelData to zencodec_types::PixelData.
fn convert_pixels(pixels: crate::PixelData) -> ZPixelData {
    match pixels {
        crate::PixelData::Rgb8(img) => ZPixelData::Rgb8(img),
        crate::PixelData::Rgba8(img) => ZPixelData::Rgba8(img),
        crate::PixelData::Rgb16(img) => ZPixelData::Rgb16(img),
        crate::PixelData::Rgba16(img) => ZPixelData::Rgba16(img),
        crate::PixelData::RgbF32(img) => ZPixelData::RgbF32(img),
        crate::PixelData::RgbaF32(img) => ZPixelData::RgbaF32(img),
        crate::PixelData::Gray8(img) => ZPixelData::Gray8(img),
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use imgref::Img;
    use zencodec_types::{Decoding, Encoding};

    #[test]
    fn encoding_rgb8() {
        let enc = PngEncoding::new();
        let pixels = vec![Rgb { r: 128, g: 64, b: 32 }; 64];
        let img = Img::new(pixels, 8, 8);
        let output = enc.encode_rgb8(img.as_ref()).unwrap();
        assert!(!output.bytes().is_empty());
        assert_eq!(output.format(), ZImageFormat::Png);
        // Verify PNG signature
        assert_eq!(
            &output.bytes()[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    #[test]
    fn encoding_rgba8() {
        let enc = PngEncoding::new();
        let pixels = vec![
            Rgba {
                r: 100,
                g: 150,
                b: 200,
                a: 128,
            };
            64
        ];
        let img = Img::new(pixels, 8, 8);
        let output = enc.encode_rgba8(img.as_ref()).unwrap();
        assert!(!output.bytes().is_empty());
    }

    #[test]
    fn encoding_gray8() {
        let enc = PngEncoding::new();
        let pixels = vec![Gray::new(128u8); 64];
        let img = Img::new(pixels, 8, 8);
        let output = enc.encode_gray8(img.as_ref()).unwrap();
        assert!(!output.bytes().is_empty());
    }

    #[test]
    fn decode_roundtrip() {
        let enc = PngEncoding::new();
        let pixels = vec![Rgb { r: 200, g: 100, b: 50 }; 64];
        let img = Img::new(pixels, 8, 8);
        let encoded = enc.encode_rgb8(img.as_ref()).unwrap();

        let dec = PngDecoding::new();
        let output = dec.decode(encoded.bytes()).unwrap();
        assert_eq!(output.info().width, 8);
        assert_eq!(output.info().height, 8);
        assert_eq!(output.info().format, ZImageFormat::Png);
    }

    #[test]
    fn probe_info() {
        let enc = PngEncoding::new();
        let pixels = vec![Rgb { r: 0, g: 0, b: 0 }; 100];
        let img = Img::new(pixels, 10, 10);
        let encoded = enc.encode_rgb8(img.as_ref()).unwrap();

        let dec = PngDecoding::new();
        let info = dec.probe(encoded.bytes()).unwrap();
        assert_eq!(info.width, 10);
        assert_eq!(info.height, 10);
        assert_eq!(info.format, ZImageFormat::Png);
    }
}
