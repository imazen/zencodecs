//! PNG codec adapter using png crate.
//!
//! Note: PNG codec requires std due to png crate's use of std::io traits.

extern crate std;

use alloc::vec::Vec;
use std::io::Cursor;

use crate::{
    CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, Limits, PixelLayout, Stop,
};

/// Probe PNG metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let cursor = Cursor::new(data);
    let decoder = png::Decoder::new(cursor);

    let reader = decoder
        .read_info()
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;

    let info = reader.info();
    let has_alpha = matches!(
        info.color_type,
        png::ColorType::Rgba | png::ColorType::GrayscaleAlpha
    );

    // Check for animation (APNG)
    let has_animation = info.animation_control.is_some();
    let frame_count = if let Some(actl) = &info.animation_control {
        actl.num_frames
    } else {
        1
    };

    // Extract ICC profile if present
    let icc_profile = info.icc_profile.as_ref().map(|p| p.to_vec());

    Ok(ImageInfo {
        width: info.width,
        height: info.height,
        format: ImageFormat::Png,
        has_alpha,
        has_animation,
        frame_count: Some(frame_count),
        icc_profile,
    })
}

/// Decode PNG to pixels.
pub(crate) fn decode(
    data: &[u8],
    output_layout: PixelLayout,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let cursor = Cursor::new(data);
    let decoder = png::Decoder::new(cursor);

    let mut reader = decoder
        .read_info()
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;

    // Get info for output
    let info = reader.info();
    let width = info.width;
    let height = info.height;
    let has_alpha = matches!(
        info.color_type,
        png::ColorType::Rgba | png::ColorType::GrayscaleAlpha
    );
    let has_animation = info.animation_control.is_some();
    let frame_count = if let Some(actl) = &info.animation_control {
        actl.num_frames
    } else {
        1
    };
    let icc_profile = info.icc_profile.as_ref().map(|p| p.to_vec());

    // Allocate buffer
    let buffer_size = reader
        .output_buffer_size()
        .ok_or_else(|| CodecError::InvalidInput("cannot determine PNG output buffer size".into()))?;
    let mut pixels = alloc::vec![0u8; buffer_size];

    // Decode frame
    let output_info = reader
        .next_frame(&mut pixels)
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;

    // Truncate to actual size
    pixels.truncate(output_info.buffer_size());

    // Get actual output color type from decoder
    let (decoded_color_type, _bit_depth) = reader.output_color_type();
    let decoded_layout = match decoded_color_type {
        png::ColorType::Rgb => PixelLayout::Rgb8,
        png::ColorType::Rgba => PixelLayout::Rgba8,
        png::ColorType::Grayscale | png::ColorType::GrayscaleAlpha => {
            // PNG decoder expands grayscale to RGB/RGBA
            if has_alpha {
                PixelLayout::Rgba8
            } else {
                PixelLayout::Rgb8
            }
        }
        png::ColorType::Indexed => {
            // Indexed color is expanded to RGB/RGBA
            if has_alpha {
                PixelLayout::Rgba8
            } else {
                PixelLayout::Rgb8
            }
        }
    };

    // Convert to requested layout if needed
    let (final_pixels, final_layout) = if decoded_layout == output_layout {
        (pixels, decoded_layout)
    } else {
        // TODO: Implement pixel format conversion
        // For now, return what the decoder gave us
        (pixels, decoded_layout)
    };

    Ok(DecodeOutput {
        pixels: final_pixels,
        width,
        height,
        layout: final_layout,
        info: ImageInfo {
            width,
            height,
            format: ImageFormat::Png,
            has_alpha,
            has_animation,
            frame_count: Some(frame_count),
            icc_profile,
        },
    })
}

/// Encode pixels to PNG.
#[allow(clippy::too_many_arguments)]
pub(crate) fn encode(
    pixels: &[u8],
    width: u32,
    height: u32,
    layout: PixelLayout,
    _quality: Option<f32>, // PNG is lossless, no quality setting
    _lossless: bool,       // PNG is always lossless
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    // Convert BGR/BGRA to RGB/RGBA if needed
    let (final_pixels, color_type) = match layout {
        PixelLayout::Rgb8 => (pixels.to_vec(), png::ColorType::Rgb),
        PixelLayout::Rgba8 => (pixels.to_vec(), png::ColorType::Rgba),
        PixelLayout::Bgr8 => {
            // Convert BGR to RGB
            let mut rgb = Vec::with_capacity(pixels.len());
            for chunk in pixels.chunks_exact(3) {
                rgb.push(chunk[2]); // R
                rgb.push(chunk[1]); // G
                rgb.push(chunk[0]); // B
            }
            (rgb, png::ColorType::Rgb)
        }
        PixelLayout::Bgra8 => {
            // Convert BGRA to RGBA
            let mut rgba = Vec::with_capacity(pixels.len());
            for chunk in pixels.chunks_exact(4) {
                rgba.push(chunk[2]); // R
                rgba.push(chunk[1]); // G
                rgba.push(chunk[0]); // B
                rgba.push(chunk[3]); // A
            }
            (rgba, png::ColorType::Rgba)
        }
    };

    // Create output buffer
    let mut output = Vec::new();
    let mut encoder = png::Encoder::new(&mut output, width, height);
    encoder.set_color(color_type);
    encoder.set_depth(png::BitDepth::Eight);

    // Write header and get writer
    let mut writer = encoder
        .write_header()
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;

    // Write pixel data
    writer
        .write_image_data(&final_pixels)
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;

    // Drop writer to flush
    drop(writer);

    Ok(EncodeOutput {
        data: output,
        format: ImageFormat::Png,
    })
}
