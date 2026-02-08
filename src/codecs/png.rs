//! PNG codec adapter using png crate.
//!
//! Note: PNG codec requires std due to png crate's use of std::io traits.

extern crate std;

use std::io::Cursor;

use crate::pixel::{ImgRef, ImgVec, Rgb, Rgba};
use crate::{CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, Limits, PixelData, Stop};

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

    let has_animation = info.animation_control.is_some();
    let frame_count = if let Some(actl) = &info.animation_control {
        actl.num_frames
    } else {
        1
    };

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
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let cursor = Cursor::new(data);
    let decoder = png::Decoder::new(cursor);

    let mut reader = decoder
        .read_info()
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;

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

    let buffer_size = reader
        .output_buffer_size()
        .ok_or_else(|| CodecError::InvalidInput("cannot determine PNG output buffer size".into()))?;
    let mut raw_pixels = alloc::vec![0u8; buffer_size];

    let output_info = reader
        .next_frame(&mut raw_pixels)
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;

    raw_pixels.truncate(output_info.buffer_size());

    let (decoded_color_type, _bit_depth) = reader.output_color_type();
    let w = width as usize;
    let h = height as usize;

    let pixels = match decoded_color_type {
        png::ColorType::Rgba => {
            let rgba: &[Rgba<u8>] = bytemuck::cast_slice(&raw_pixels);
            PixelData::Rgba8(ImgVec::new(rgba.to_vec(), w, h))
        }
        png::ColorType::Rgb => {
            let rgb: &[Rgb<u8>] = bytemuck::cast_slice(&raw_pixels);
            PixelData::Rgb8(ImgVec::new(rgb.to_vec(), w, h))
        }
        png::ColorType::GrayscaleAlpha => {
            // GA → RGBA
            let rgba: alloc::vec::Vec<Rgba<u8>> = raw_pixels
                .chunks_exact(2)
                .map(|ga| Rgba {
                    r: ga[0],
                    g: ga[0],
                    b: ga[0],
                    a: ga[1],
                })
                .collect();
            PixelData::Rgba8(ImgVec::new(rgba, w, h))
        }
        png::ColorType::Grayscale => {
            let gray: alloc::vec::Vec<rgb::Gray<u8>> = raw_pixels
                .iter()
                .map(|&g| rgb::Gray(g))
                .collect();
            PixelData::Gray8(ImgVec::new(gray, w, h))
        }
        png::ColorType::Indexed => {
            // png crate expands indexed to RGB/RGBA
            if has_alpha {
                let rgba: &[Rgba<u8>] = bytemuck::cast_slice(&raw_pixels);
                PixelData::Rgba8(ImgVec::new(rgba.to_vec(), w, h))
            } else {
                let rgb: &[Rgb<u8>] = bytemuck::cast_slice(&raw_pixels);
                PixelData::Rgb8(ImgVec::new(rgb.to_vec(), w, h))
            }
        }
    };

    Ok(DecodeOutput {
        pixels,
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

/// Encode RGB8 pixels to PNG.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let width = img.width() as u32;
    let height = img.height() as u32;
    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());

    let mut output = Vec::new();
    let mut encoder = png::Encoder::new(&mut output, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder
        .write_header()
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;

    writer
        .write_image_data(bytes)
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;

    drop(writer);

    Ok(EncodeOutput {
        data: output,
        format: ImageFormat::Png,
    })
}

/// Encode RGBA8 pixels to PNG.
pub(crate) fn encode_rgba8(
    img: ImgRef<Rgba<u8>>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let width = img.width() as u32;
    let height = img.height() as u32;
    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());

    let mut output = Vec::new();
    let mut encoder = png::Encoder::new(&mut output, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder
        .write_header()
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;

    writer
        .write_image_data(bytes)
        .map_err(|e| CodecError::from_codec(ImageFormat::Png, e))?;

    drop(writer);

    Ok(EncodeOutput {
        data: output,
        format: ImageFormat::Png,
    })
}
