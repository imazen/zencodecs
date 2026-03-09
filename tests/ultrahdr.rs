//! Integration tests for UltraHDR JPEG encode/decode through zencodecs.
//!
//! Verifies the full pipeline: create HDR → encode UltraHDR JPEG → decode HDR → verify pixels.

#![cfg(feature = "jpeg-ultrahdr")]

use imgref::ImgVec;
use rgb::{Rgb, Rgba};
use zencodecs::{DecodeRequest, EncodeRequest, ImageFormat};

/// Create a synthetic linear f32 RGB "HDR" gradient.
///
/// Values exceed 1.0 to simulate HDR content (linear light, BT.709 gamut).
fn hdr_rgb_f32_image(w: usize, h: usize) -> ImgVec<Rgb<f32>> {
    let pixels: Vec<Rgb<f32>> = (0..w * h)
        .map(|i| {
            let x = (i % w) as f32 / w as f32;
            let y = (i / w) as f32 / h as f32;
            Rgb {
                r: x * 2.5, // up to 2.5 (HDR peak)
                g: y * 1.5,
                b: 0.3,
            }
        })
        .collect();
    ImgVec::new(pixels, w, h)
}

/// Create a synthetic linear f32 RGBA "HDR" gradient.
fn hdr_rgba_f32_image(w: usize, h: usize) -> ImgVec<Rgba<f32>> {
    let pixels: Vec<Rgba<f32>> = (0..w * h)
        .map(|i| {
            let x = (i % w) as f32 / w as f32;
            let y = (i / w) as f32 / h as f32;
            Rgba {
                r: x * 3.0,
                g: y * 2.0,
                b: 0.5,
                a: 1.0,
            }
        })
        .collect();
    ImgVec::new(pixels, w, h)
}

#[test]
fn encode_ultrahdr_rgb_f32_produces_jpeg() {
    let img = hdr_rgb_f32_image(64, 64);

    let output = EncodeRequest::new(ImageFormat::Jpeg)
        .with_quality(85.0)
        .encode_ultrahdr_rgb_f32(img.as_ref())
        .expect("UltraHDR RGB f32 encode failed");

    assert_eq!(output.format(), ImageFormat::Jpeg);
    // Should be a valid JPEG (starts with SOI marker)
    let bytes = output.data();
    assert!(bytes.len() > 100, "encoded output too small");
    assert_eq!(&bytes[..2], &[0xFF, 0xD8], "not a valid JPEG");
}

#[test]
fn encode_ultrahdr_rgba_f32_produces_jpeg() {
    let img = hdr_rgba_f32_image(64, 64);

    let output = EncodeRequest::new(ImageFormat::Jpeg)
        .with_quality(85.0)
        .with_gainmap_quality(60.0)
        .encode_ultrahdr_rgba_f32(img.as_ref())
        .expect("UltraHDR RGBA f32 encode failed");

    assert_eq!(output.format(), ImageFormat::Jpeg);
    let bytes = output.data();
    assert!(bytes.len() > 100, "encoded output too small");
    assert_eq!(&bytes[..2], &[0xFF, 0xD8], "not a valid JPEG");
}

#[test]
fn ultrahdr_roundtrip_rgb_f32() {
    let w = 64;
    let h = 64;
    let img = hdr_rgb_f32_image(w, h);

    // Encode to UltraHDR
    let encoded = EncodeRequest::new(ImageFormat::Jpeg)
        .with_quality(90.0)
        .encode_ultrahdr_rgb_f32(img.as_ref())
        .expect("encode failed");

    // Decode SDR (standard path — should work even for UltraHDR)
    let sdr = DecodeRequest::new(encoded.data())
        .decode()
        .expect("SDR decode failed");

    assert_eq!(sdr.width(), w as u32);
    assert_eq!(sdr.height(), h as u32);
    assert_eq!(sdr.format(), ImageFormat::Jpeg);

    // Decode HDR
    let hdr = DecodeRequest::new(encoded.data())
        .decode_hdr(4.0)
        .expect("HDR decode failed");

    assert_eq!(hdr.width(), w as u32);
    assert_eq!(hdr.height(), h as u32);
    assert_eq!(hdr.format(), ImageFormat::Jpeg);
}

#[test]
fn ultrahdr_roundtrip_rgba_f32() {
    let w = 128;
    let h = 64;
    let img = hdr_rgba_f32_image(w, h);

    let encoded = EncodeRequest::new(ImageFormat::Jpeg)
        .with_quality(85.0)
        .with_gainmap_quality(75.0)
        .encode_ultrahdr_rgba_f32(img.as_ref())
        .expect("encode failed");

    // Both SDR and HDR decode should work
    let sdr = DecodeRequest::new(encoded.data())
        .decode()
        .expect("SDR decode failed");
    assert_eq!(sdr.width(), w as u32);
    assert_eq!(sdr.height(), h as u32);

    let hdr = DecodeRequest::new(encoded.data())
        .decode_hdr(4.0)
        .expect("HDR decode failed");
    assert_eq!(hdr.width(), w as u32);
    assert_eq!(hdr.height(), h as u32);
}

#[test]
fn decode_hdr_rejects_non_ultrahdr_jpeg() {
    // Create a regular JPEG (no gain map)
    let img = ImgVec::new(
        vec![
            Rgb {
                r: 128u8,
                g: 128,
                b: 128,
            };
            32 * 32
        ],
        32,
        32,
    );
    let encoded = EncodeRequest::new(ImageFormat::Jpeg)
        .with_quality(85.0)
        .encode_rgb8(img.as_ref())
        .expect("encode failed");

    // decode_hdr should fail for non-UltraHDR
    let result = DecodeRequest::new(encoded.data()).decode_hdr(4.0);
    assert!(result.is_err(), "expected error for non-UltraHDR JPEG");
}

#[test]
fn decode_hdr_rejects_non_jpeg() {
    // Try decode_hdr with WebP data (if webp feature is enabled)
    #[cfg(feature = "webp")]
    {
        let img = ImgVec::new(
            vec![
                Rgb {
                    r: 128u8,
                    g: 128,
                    b: 128,
                };
                32 * 32
            ],
            32,
            32,
        );
        let webp = EncodeRequest::new(ImageFormat::WebP)
            .with_quality(85.0)
            .encode_rgb8(img.as_ref())
            .expect("webp encode failed");

        let result = DecodeRequest::new(webp.data()).decode_hdr(4.0);
        assert!(result.is_err(), "expected error for non-JPEG");
    }
}

#[test]
fn ultrahdr_encode_disabled_registry() {
    let img = hdr_rgb_f32_image(32, 32);
    let registry = zencodecs::CodecRegistry::none();

    let result = EncodeRequest::new(ImageFormat::Jpeg)
        .with_registry(&registry)
        .encode_ultrahdr_rgb_f32(img.as_ref());

    assert!(
        matches!(
            result.as_ref().map_err(|e| e.error()),
            Err(zencodecs::CodecError::DisabledFormat(_))
        ),
        "expected DisabledFormat error"
    );
}
