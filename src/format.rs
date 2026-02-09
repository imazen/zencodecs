//! Image format detection and metadata.

/// Supported image formats.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Jpeg,
    WebP,
    Gif,
    Png,
    Avif,
    Jxl,
}

impl ImageFormat {
    /// Detect format from magic bytes. Returns None if unrecognized.
    ///
    /// Checks the first few bytes of the data for known format signatures.
    pub fn detect(data: &[u8]) -> Option<Self> {
        // JPEG: FF D8 FF
        if data.len() >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
            return Some(ImageFormat::Jpeg);
        }

        // PNG: 89 50 4E 47 0D 0A 1A 0A
        if data.len() >= 8
            && data[0] == 0x89
            && data[1] == 0x50
            && data[2] == 0x4E
            && data[3] == 0x47
            && data[4] == 0x0D
            && data[5] == 0x0A
            && data[6] == 0x1A
            && data[7] == 0x0A
        {
            return Some(ImageFormat::Png);
        }

        // GIF: "GIF87a" or "GIF89a"
        if data.len() >= 6
            && data[0] == b'G'
            && data[1] == b'I'
            && data[2] == b'F'
            && data[3] == b'8'
            && (data[4] == b'7' || data[4] == b'9')
            && data[5] == b'a'
        {
            return Some(ImageFormat::Gif);
        }

        // WebP: "RIFF....WEBP"
        if data.len() >= 12
            && data[0] == b'R'
            && data[1] == b'I'
            && data[2] == b'F'
            && data[3] == b'F'
            && data[8] == b'W'
            && data[9] == b'E'
            && data[10] == b'B'
            && data[11] == b'P'
        {
            return Some(ImageFormat::WebP);
        }

        // AVIF: ftyp box with compatible brands
        // Format: offset 4: "ftyp", then major brand (avif, avis)
        if data.len() >= 12 && &data[4..8] == b"ftyp" {
            let brand = &data[8..12];
            if brand == b"avif" || brand == b"avis" {
                return Some(ImageFormat::Avif);
            }
        }

        // JPEG XL codestream: FF 0A
        if data.len() >= 2 && data[0] == 0xFF && data[1] == 0x0A {
            return Some(ImageFormat::Jxl);
        }

        // JPEG XL container: 00 00 00 0C 4A 58 4C 20 0D 0A 87 0A
        if data.len() >= 12
            && data[0..4] == [0x00, 0x00, 0x00, 0x0C]
            && data[4..8] == [b'J', b'X', b'L', b' ']
            && data[8..12] == [0x0D, 0x0A, 0x87, 0x0A]
        {
            return Some(ImageFormat::Jxl);
        }

        None
    }

    /// Detect format from file extension (case-insensitive).
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" | "jpe" | "jfif" => Some(ImageFormat::Jpeg),
            "webp" => Some(ImageFormat::WebP),
            "gif" => Some(ImageFormat::Gif),
            "png" => Some(ImageFormat::Png),
            "avif" => Some(ImageFormat::Avif),
            "jxl" => Some(ImageFormat::Jxl),
            _ => None,
        }
    }

    /// MIME type string.
    pub fn mime_type(self) -> &'static str {
        match self {
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Gif => "image/gif",
            ImageFormat::Png => "image/png",
            ImageFormat::Avif => "image/avif",
            ImageFormat::Jxl => "image/jxl",
        }
    }

    /// Common file extensions.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            ImageFormat::Jpeg => &["jpg", "jpeg", "jpe", "jfif"],
            ImageFormat::WebP => &["webp"],
            ImageFormat::Gif => &["gif"],
            ImageFormat::Png => &["png"],
            ImageFormat::Avif => &["avif"],
            ImageFormat::Jxl => &["jxl"],
        }
    }

    /// Whether this format supports lossy encoding.
    pub fn supports_lossy(self) -> bool {
        match self {
            ImageFormat::Jpeg => true,
            ImageFormat::WebP => true,
            ImageFormat::Gif => false,
            ImageFormat::Png => false,
            ImageFormat::Avif => true,
            ImageFormat::Jxl => true,
        }
    }

    /// Whether this format supports lossless encoding.
    pub fn supports_lossless(self) -> bool {
        match self {
            ImageFormat::Jpeg => false,
            ImageFormat::WebP => true,
            ImageFormat::Gif => true,
            ImageFormat::Png => true,
            ImageFormat::Avif => false,
            ImageFormat::Jxl => true,
        }
    }

    /// Whether this format supports animation.
    pub fn supports_animation(self) -> bool {
        match self {
            ImageFormat::Jpeg => false,
            ImageFormat::WebP => true,
            ImageFormat::Gif => true,
            ImageFormat::Png => false, // APNG exists but we don't support it
            ImageFormat::Avif => false,
            ImageFormat::Jxl => true,
        }
    }

    /// Recommended bytes to fetch for probing any format.
    ///
    /// 4096 bytes is enough for all formats including JPEG (which may have
    /// large EXIF/APP segments before the SOF marker).
    pub const RECOMMENDED_PROBE_BYTES: usize = 4096;

    /// Minimum bytes needed for reliable dimension probing of this format.
    ///
    /// With fewer bytes, format detection may succeed but dimensions may be
    /// `None` in the `ProbeResult`.
    pub fn min_probe_bytes(self) -> usize {
        match self {
            ImageFormat::Png => 33,    // 8 sig + 25 IHDR
            ImageFormat::Gif => 13,    // 6 header + 7 LSD
            ImageFormat::WebP => 30,   // RIFF(12) + chunk header + VP8X dims
            ImageFormat::Jpeg => 2048, // SOF can follow large EXIF/APP segments
            ImageFormat::Avif => 512,  // ISOBMFF box traversal (ftyp + meta)
            ImageFormat::Jxl => 256,   // codestream header or container + jxlc
        }
    }

    /// Whether this format supports alpha channel.
    pub fn supports_alpha(self) -> bool {
        match self {
            ImageFormat::Jpeg => false,
            ImageFormat::WebP => true,
            ImageFormat::Gif => true,
            ImageFormat::Png => true,
            ImageFormat::Avif => true,
            ImageFormat::Jxl => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_jpeg() {
        let data = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(ImageFormat::detect(&data), Some(ImageFormat::Jpeg));
    }

    #[test]
    fn detect_png() {
        let data = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
        ];
        assert_eq!(ImageFormat::detect(&data), Some(ImageFormat::Png));
    }

    #[test]
    fn detect_gif() {
        let data = b"GIF89a\x00\x00\x00\x00\x00\x00";
        assert_eq!(ImageFormat::detect(data), Some(ImageFormat::Gif));
    }

    #[test]
    fn detect_webp() {
        let data = b"RIFF\x00\x00\x00\x00WEBP";
        assert_eq!(ImageFormat::detect(data), Some(ImageFormat::WebP));
    }

    #[test]
    fn detect_avif() {
        let data = b"\x00\x00\x00\x18ftypavif";
        assert_eq!(ImageFormat::detect(data), Some(ImageFormat::Avif));
    }

    #[test]
    fn detect_jxl_codestream() {
        let data = [0xFF, 0x0A, 0x00, 0x00];
        assert_eq!(ImageFormat::detect(&data), Some(ImageFormat::Jxl));
    }

    #[test]
    fn detect_jxl_container() {
        let data = [0x00, 0x00, 0x00, 0x0C, b'J', b'X', b'L', b' ', 0x0D, 0x0A, 0x87, 0x0A];
        assert_eq!(ImageFormat::detect(&data), Some(ImageFormat::Jxl));
    }

    #[test]
    fn detect_too_short() {
        let data = [0xFF, 0xD8];
        assert_eq!(ImageFormat::detect(&data), None);
    }

    #[test]
    fn from_extension_case_insensitive() {
        assert_eq!(ImageFormat::from_extension("JPG"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_extension("WebP"), Some(ImageFormat::WebP));
        assert_eq!(ImageFormat::from_extension("unknown"), None);
    }
}
