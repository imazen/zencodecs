//! Typed pixel buffer definitions.
//!
//! Uses `imgref::ImgVec` for 2D pixel data with typed pixels from the `rgb` crate.

pub use imgref::{Img, ImgRef, ImgVec};
pub use rgb::{Gray, Rgb, Rgba};

/// Decoded pixel data in a typed buffer.
///
/// The variant determines both the pixel format and precision.
/// Width and height are embedded in the `ImgVec`.
#[non_exhaustive]
pub enum PixelData {
    Rgb8(ImgVec<Rgb<u8>>),
    Rgba8(ImgVec<Rgba<u8>>),
    Rgb16(ImgVec<Rgb<u16>>),
    Rgba16(ImgVec<Rgba<u16>>),
    RgbF32(ImgVec<Rgb<f32>>),
    RgbaF32(ImgVec<Rgba<f32>>),
    Gray8(ImgVec<Gray<u8>>),
}

impl PixelData {
    /// Image width in pixels.
    pub fn width(&self) -> u32 {
        match self {
            PixelData::Rgb8(img) => img.width() as u32,
            PixelData::Rgba8(img) => img.width() as u32,
            PixelData::Rgb16(img) => img.width() as u32,
            PixelData::Rgba16(img) => img.width() as u32,
            PixelData::RgbF32(img) => img.width() as u32,
            PixelData::RgbaF32(img) => img.width() as u32,
            PixelData::Gray8(img) => img.width() as u32,
        }
    }

    /// Image height in pixels.
    pub fn height(&self) -> u32 {
        match self {
            PixelData::Rgb8(img) => img.height() as u32,
            PixelData::Rgba8(img) => img.height() as u32,
            PixelData::Rgb16(img) => img.height() as u32,
            PixelData::Rgba16(img) => img.height() as u32,
            PixelData::RgbF32(img) => img.height() as u32,
            PixelData::RgbaF32(img) => img.height() as u32,
            PixelData::Gray8(img) => img.height() as u32,
        }
    }

    /// Whether this pixel data has an alpha channel.
    pub fn has_alpha(&self) -> bool {
        matches!(
            self,
            PixelData::Rgba8(_) | PixelData::Rgba16(_) | PixelData::RgbaF32(_)
        )
    }

    /// Convert to RGBA8, allocating a new buffer.
    ///
    /// Gray8 is expanded to RGBA with R=G=B=gray, A=255.
    /// Rgb8 gets A=255 added. Rgba8 is cloned.
    /// Higher-precision formats are clamped/truncated to 8-bit.
    pub fn to_rgba8(&self) -> ImgVec<Rgba<u8>> {
        match self {
            PixelData::Rgba8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                ImgVec::new(buf.into_owned(), w, h)
            }
            PixelData::Rgb8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: alloc::vec::Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: p.r,
                        g: p.g,
                        b: p.b,
                        a: 255,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Gray8(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: alloc::vec::Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: p.value(),
                        g: p.value(),
                        b: p.value(),
                        a: 255,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Rgba16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: alloc::vec::Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: (p.r >> 8) as u8,
                        g: (p.g >> 8) as u8,
                        b: (p.b >> 8) as u8,
                        a: (p.a >> 8) as u8,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::Rgb16(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: alloc::vec::Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: (p.r >> 8) as u8,
                        g: (p.g >> 8) as u8,
                        b: (p.b >> 8) as u8,
                        a: 255,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::RgbF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: alloc::vec::Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: (p.r.clamp(0.0, 1.0) * 255.0) as u8,
                        g: (p.g.clamp(0.0, 1.0) * 255.0) as u8,
                        b: (p.b.clamp(0.0, 1.0) * 255.0) as u8,
                        a: 255,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
            PixelData::RgbaF32(img) => {
                let (buf, w, h) = img.as_ref().to_contiguous_buf();
                let rgba: alloc::vec::Vec<Rgba<u8>> = buf
                    .iter()
                    .map(|p| Rgba {
                        r: (p.r.clamp(0.0, 1.0) * 255.0) as u8,
                        g: (p.g.clamp(0.0, 1.0) * 255.0) as u8,
                        b: (p.b.clamp(0.0, 1.0) * 255.0) as u8,
                        a: (p.a.clamp(0.0, 1.0) * 255.0) as u8,
                    })
                    .collect();
                ImgVec::new(rgba, w, h)
            }
        }
    }

    /// Get the raw pixel data as a byte slice.
    ///
    /// Returns the underlying contiguous buffer as bytes.
    pub fn as_bytes(&self) -> alloc::vec::Vec<u8> {
        use rgb::ComponentBytes;
        match self {
            PixelData::Rgb8(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::Rgba8(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::Rgb16(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::Rgba16(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::RgbF32(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::RgbaF32(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
            PixelData::Gray8(img) => {
                let (buf, _, _) = img.as_ref().to_contiguous_buf();
                buf.as_bytes().to_vec()
            }
        }
    }
}

impl core::fmt::Debug for PixelData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let variant = match self {
            PixelData::Rgb8(_) => "Rgb8",
            PixelData::Rgba8(_) => "Rgba8",
            PixelData::Rgb16(_) => "Rgb16",
            PixelData::Rgba16(_) => "Rgba16",
            PixelData::RgbF32(_) => "RgbF32",
            PixelData::RgbaF32(_) => "RgbaF32",
            PixelData::Gray8(_) => "Gray8",
        };
        write!(f, "PixelData::{}({}x{})", variant, self.width(), self.height())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_data_dimensions() {
        let img = ImgVec::new(vec![Rgb { r: 0u8, g: 0, b: 0 }; 100], 10, 10);
        let data = PixelData::Rgb8(img);
        assert_eq!(data.width(), 10);
        assert_eq!(data.height(), 10);
        assert!(!data.has_alpha());
    }

    #[test]
    fn pixel_data_rgba_has_alpha() {
        let img = ImgVec::new(
            vec![Rgba { r: 0u8, g: 0, b: 0, a: 255 }; 4],
            2,
            2,
        );
        let data = PixelData::Rgba8(img);
        assert!(data.has_alpha());
    }

    #[test]
    fn rgb8_to_rgba8() {
        let img = ImgVec::new(
            vec![Rgb { r: 10u8, g: 20, b: 30 }; 4],
            2,
            2,
        );
        let data = PixelData::Rgb8(img);
        let rgba = data.to_rgba8();
        assert_eq!(rgba.width(), 2);
        assert_eq!(rgba.height(), 2);
        let px = &rgba.buf()[0];
        assert_eq!(px.r, 10);
        assert_eq!(px.g, 20);
        assert_eq!(px.b, 30);
        assert_eq!(px.a, 255);
    }
}
