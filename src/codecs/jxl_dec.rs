//! JPEG XL decode adapter using the jxl crate (jxl-rs).

use alloc::vec;

use crate::{CodecError, DecodeOutput, ImageFormat, ImageInfo, Limits, PixelData, Stop};

use jxl::api::{
    JxlDecoder, JxlDecoderOptions, JxlOutputBuffer, JxlPixelFormat, ProcessingResult,
};
use jxl::headers::extra_channels::ExtraChannel;

fn map_err(e: jxl::error::Error) -> CodecError {
    CodecError::from_codec(ImageFormat::Jxl, e)
}

/// Probe JXL metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let options = JxlDecoderOptions::default();
    let decoder = JxlDecoder::new(options);

    let mut input = data;
    let decoder = match decoder.process(&mut input).map_err(map_err)? {
        ProcessingResult::Complete { result } => result,
        ProcessingResult::NeedsMoreInput { .. } => {
            return Err(CodecError::InvalidInput(
                "JXL: insufficient data for header".into(),
            ));
        }
    };

    let info = decoder.basic_info();
    let (width, height) = info.size;
    let has_alpha = info
        .extra_channels
        .iter()
        .any(|ec| matches!(ec.ec_type, ExtraChannel::Alpha));
    let has_animation = info.animation.is_some();

    Ok(ImageInfo {
        width: width as u32,
        height: height as u32,
        format: ImageFormat::Jxl,
        has_alpha,
        has_animation,
        frame_count: None,
        icc_profile: None,
        exif: None,
        xmp: None,
    })
}

/// Decode JXL to pixels.
pub(crate) fn decode(
    data: &[u8],
    limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let mut options = JxlDecoderOptions::default();

    // Map pixel_limit from zencodecs Limits
    if let Some(lim) = limits {
        if let Some(max_px) = lim.max_pixels {
            options.pixel_limit = Some(max_px as usize);
        }
    }

    let decoder = JxlDecoder::new(options);

    // Phase 1: parse header → WithImageInfo
    let mut input = data;
    let mut decoder = match decoder.process(&mut input).map_err(map_err)? {
        ProcessingResult::Complete { result } => result,
        ProcessingResult::NeedsMoreInput { .. } => {
            return Err(CodecError::InvalidInput(
                "JXL: insufficient data for header".into(),
            ));
        }
    };

    let info = decoder.basic_info();
    let (width, height) = info.size;
    let has_alpha = info
        .extra_channels
        .iter()
        .any(|ec| matches!(ec.ec_type, ExtraChannel::Alpha));
    let has_animation = info.animation.is_some();

    // Validate dimensions against limits
    if let Some(lim) = limits {
        let bpp: u32 = if has_alpha { 4 } else { 3 };
        lim.validate(width as u32, height as u32, bpp)?;
    }

    // Count non-alpha extra channels (for the pixel format)
    let num_extra = info
        .extra_channels
        .iter()
        .filter(|ec| !matches!(ec.ec_type, ExtraChannel::Alpha))
        .count();

    // Request RGBA8 output (alpha channel is interleaved automatically)
    let pixel_format = JxlPixelFormat::rgba8(num_extra);
    decoder.set_pixel_format(pixel_format);

    // Phase 2: process to WithFrameInfo
    let decoder = match decoder.process(&mut input).map_err(map_err)? {
        ProcessingResult::Complete { result } => result,
        ProcessingResult::NeedsMoreInput { .. } => {
            return Err(CodecError::InvalidInput(
                "JXL: insufficient data for frame".into(),
            ));
        }
    };

    // Phase 3: decode pixels into output buffer
    let bytes_per_row = width * 4; // RGBA8 = 4 bytes per pixel
    let buf_size = bytes_per_row * height;
    let mut buf = vec![0u8; buf_size];

    let output = JxlOutputBuffer::new(&mut buf, height, bytes_per_row);
    let _decoder = match decoder.process(&mut input, &mut [output]).map_err(map_err)? {
        ProcessingResult::Complete { result } => result,
        ProcessingResult::NeedsMoreInput { .. } => {
            return Err(CodecError::InvalidInput(
                "JXL: insufficient data for pixels".into(),
            ));
        }
    };

    // Convert RGBA8 buffer to PixelData
    let pixels = if has_alpha {
        let rgba_pixels: alloc::vec::Vec<rgb::Rgba<u8>> = buf
            .chunks_exact(4)
            .map(|c| rgb::Rgba {
                r: c[0],
                g: c[1],
                b: c[2],
                a: c[3],
            })
            .collect();
        PixelData::Rgba8(imgref::ImgVec::new(rgba_pixels, width, height))
    } else {
        // Strip alpha channel since the image has no alpha
        let rgb_pixels: alloc::vec::Vec<rgb::Rgb<u8>> = buf
            .chunks_exact(4)
            .map(|c| rgb::Rgb {
                r: c[0],
                g: c[1],
                b: c[2],
            })
            .collect();
        PixelData::Rgb8(imgref::ImgVec::new(rgb_pixels, width, height))
    };

    Ok(DecodeOutput {
        pixels,
        info: ImageInfo {
            width: width as u32,
            height: height as u32,
            format: ImageFormat::Jxl,
            has_alpha,
            has_animation,
            frame_count: None,
            icc_profile: None,
            exif: None,
            xmp: None,
        },
        #[cfg(feature = "jpeg")]
        jpeg_extras: None,
    })
}
