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

impl PixelLayout {
    /// Whether this layout uses BGR channel order.
    pub fn is_bgr(self) -> bool {
        matches!(self, PixelLayout::Bgr8 | PixelLayout::Bgra8)
    }
}

/// Convert pixel data to RGB-order layout, swapping B and R channels if needed.
///
/// Returns `(converted_bytes, new_layout)`. If already RGB-order, returns a copy.
/// BGR8 → RGB8, BGRA8 → RGBA8, RGB8/RGBA8 → unchanged (cloned).
pub(crate) fn to_rgb_order(pixels: &[u8], layout: PixelLayout) -> (alloc::vec::Vec<u8>, PixelLayout) {
    match layout {
        PixelLayout::Rgb8 | PixelLayout::Rgba8 => (pixels.to_vec(), layout),
        PixelLayout::Bgr8 => {
            let mut rgb = alloc::vec::Vec::with_capacity(pixels.len());
            for chunk in pixels.chunks_exact(3) {
                rgb.push(chunk[2]); // R
                rgb.push(chunk[1]); // G
                rgb.push(chunk[0]); // B
            }
            (rgb, PixelLayout::Rgb8)
        }
        PixelLayout::Bgra8 => {
            let mut rgba = alloc::vec::Vec::with_capacity(pixels.len());
            for chunk in pixels.chunks_exact(4) {
                rgba.push(chunk[2]); // R
                rgba.push(chunk[1]); // G
                rgba.push(chunk[0]); // B
                rgba.push(chunk[3]); // A
            }
            (rgba, PixelLayout::Rgba8)
        }
    }
}

/// Convert pixel data to RGBA8, adding alpha=255 and swapping channels as needed.
///
/// All layouts produce RGBA8 output.
pub(crate) fn to_rgba(pixels: &[u8], layout: PixelLayout) -> alloc::vec::Vec<u8> {
    match layout {
        PixelLayout::Rgba8 => pixels.to_vec(),
        PixelLayout::Rgb8 => {
            let mut rgba = alloc::vec::Vec::with_capacity(pixels.len() / 3 * 4);
            for chunk in pixels.chunks_exact(3) {
                rgba.extend_from_slice(chunk);
                rgba.push(255);
            }
            rgba
        }
        PixelLayout::Bgra8 => {
            let mut rgba = alloc::vec::Vec::with_capacity(pixels.len());
            for chunk in pixels.chunks_exact(4) {
                rgba.push(chunk[2]); // R
                rgba.push(chunk[1]); // G
                rgba.push(chunk[0]); // B
                rgba.push(chunk[3]); // A
            }
            rgba
        }
        PixelLayout::Bgr8 => {
            let mut rgba = alloc::vec::Vec::with_capacity(pixels.len() / 3 * 4);
            for chunk in pixels.chunks_exact(3) {
                rgba.push(chunk[2]); // R
                rgba.push(chunk[1]); // G
                rgba.push(chunk[0]); // B
                rgba.push(255);
            }
            rgba
        }
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
