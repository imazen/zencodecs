//! Pixel layout definitions.

/// Pixel memory layout.
///
/// All layouts are 8-bit per channel. Only layouts that ALL codecs can handle
/// via the unified API are included here. Format-specific layouts (like YUV420
/// or Gray8) require using the codec directly.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PixelLayout {
    /// RGB, 8 bits per channel, 24 bits per pixel.
    Rgb8,
    /// RGBA, 8 bits per channel, 32 bits per pixel.
    #[default]
    Rgba8,
    /// BGR, 8 bits per channel, 24 bits per pixel.
    Bgr8,
    /// BGRA, 8 bits per channel, 32 bits per pixel.
    Bgra8,
}

impl PixelLayout {
    /// Number of bytes per pixel.
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            PixelLayout::Rgb8 | PixelLayout::Bgr8 => 3,
            PixelLayout::Rgba8 | PixelLayout::Bgra8 => 4,
        }
    }

    /// Whether this layout includes an alpha channel.
    pub fn has_alpha(self) -> bool {
        match self {
            PixelLayout::Rgb8 | PixelLayout::Bgr8 => false,
            PixelLayout::Rgba8 | PixelLayout::Bgra8 => true,
        }
    }

    /// Minimum buffer size required for an image with this layout.
    ///
    /// Returns None on overflow.
    pub fn min_buffer_size(self, width: u32, height: u32) -> Option<usize> {
        let pixels = (width as u64).checked_mul(height as u64)?;
        let bytes = pixels.checked_mul(self.bytes_per_pixel() as u64)?;
        bytes.try_into().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_per_pixel() {
        assert_eq!(PixelLayout::Rgb8.bytes_per_pixel(), 3);
        assert_eq!(PixelLayout::Rgba8.bytes_per_pixel(), 4);
        assert_eq!(PixelLayout::Bgr8.bytes_per_pixel(), 3);
        assert_eq!(PixelLayout::Bgra8.bytes_per_pixel(), 4);
    }

    #[test]
    fn has_alpha() {
        assert!(!PixelLayout::Rgb8.has_alpha());
        assert!(PixelLayout::Rgba8.has_alpha());
        assert!(!PixelLayout::Bgr8.has_alpha());
        assert!(PixelLayout::Bgra8.has_alpha());
    }

    #[test]
    fn min_buffer_size() {
        assert_eq!(PixelLayout::Rgba8.min_buffer_size(100, 100), Some(40000));
        assert_eq!(PixelLayout::Rgb8.min_buffer_size(100, 100), Some(30000));
        // Overflow check
        assert_eq!(PixelLayout::Rgba8.min_buffer_size(u32::MAX, u32::MAX), None);
    }
}
