//! JPEG XL decode adapter using jxl-oxide.

use alloc::vec::Vec;

use crate::{CodecError, DecodeOutput, ImageFormat, ImageInfo, Limits, PixelData, Stop};

use jxl_oxide::{JxlImage, PixelFormat};

fn map_err(e: Box<dyn core::error::Error + Send + Sync>) -> CodecError {
    CodecError::Codec {
        format: ImageFormat::Jxl,
        source: e,
    }
}

/// Probe JXL metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let image = JxlImage::builder()
        .read(std::io::Cursor::new(data))
        .map_err(map_err)?;

    let width = image.width();
    let height = image.height();
    let pixel_format = image.pixel_format();
    let has_alpha = pixel_format.has_alpha();
    let has_animation = image.image_header().metadata.animation.is_some();

    // Extract ICC profile
    let icc_profile = image.original_icc().map(|icc| icc.to_vec());

    // Extract EXIF
    let exif = image
        .aux_boxes()
        .first_exif()
        .ok()
        .and_then(|d| match d {
            jxl_oxide::AuxBoxData::Data(raw_exif) => Some(raw_exif.payload().to_vec()),
            _ => None,
        });

    // Extract XMP
    let xmp = match image.aux_boxes().first_xml() {
        jxl_oxide::AuxBoxData::Data(xml) => Some(xml.to_vec()),
        _ => None,
    };

    Ok(ImageInfo {
        width,
        height,
        format: ImageFormat::Jxl,
        has_alpha,
        has_animation,
        frame_count: if has_animation {
            Some(image.num_loaded_keyframes() as u32)
        } else {
            None
        },
        icc_profile,
        exif,
        xmp,
    })
}

/// Decode JXL to pixels.
pub(crate) fn decode(
    data: &[u8],
    limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let mut builder = JxlImage::builder();

    // Map memory limit from zencodecs Limits to AllocTracker
    if let Some(lim) = limits {
        if let Some(max_mem) = lim.max_memory_bytes {
            builder = builder.alloc_tracker(jxl_oxide::AllocTracker::with_limit(max_mem as usize));
        }
    }

    let image = builder
        .read(std::io::Cursor::new(data))
        .map_err(map_err)?;

    let width = image.width();
    let height = image.height();
    let pixel_format = image.pixel_format();
    let has_alpha = pixel_format.has_alpha();
    let has_animation = image.image_header().metadata.animation.is_some();

    // Validate dimensions against limits
    if let Some(lim) = limits {
        let bpp: u32 = if has_alpha { 4 } else { 3 };
        lim.validate(width, height, bpp)?;
    }

    // Render first frame
    if image.num_loaded_keyframes() == 0 {
        return Err(CodecError::InvalidInput("JXL: no frames found".into()));
    }

    let render = image.render_frame(0).map_err(map_err)?;

    // Use ImageStream to write u8 samples, which handles pixel format conversion
    let mut stream = render.stream();
    let channels = stream.channels() as usize;

    // Allocate buffer for the stream output
    let total_samples = width as usize * height as usize * channels;
    let mut buf: Vec<u8> = alloc::vec![0u8; total_samples];

    // Write all samples
    let written = stream.write_to_buffer(&mut buf);
    debug_assert_eq!(written, total_samples);

    // Extract ICC profile
    let icc_profile = image.original_icc().map(|icc| icc.to_vec());

    // Extract EXIF
    let exif = image
        .aux_boxes()
        .first_exif()
        .ok()
        .and_then(|d| match d {
            jxl_oxide::AuxBoxData::Data(raw_exif) => Some(raw_exif.payload().to_vec()),
            _ => None,
        });

    // Extract XMP
    let xmp = match image.aux_boxes().first_xml() {
        jxl_oxide::AuxBoxData::Data(xml) => Some(xml.to_vec()),
        _ => None,
    };

    let info = ImageInfo {
        width,
        height,
        format: ImageFormat::Jxl,
        has_alpha,
        has_animation,
        frame_count: if has_animation {
            Some(image.num_loaded_keyframes() as u32)
        } else {
            None
        },
        icc_profile,
        exif,
        xmp,
    };

    // Convert buffer to PixelData based on pixel format
    let pixels = match pixel_format {
        PixelFormat::Rgba => {
            let rgba_pixels: Vec<rgb::Rgba<u8>> = buf
                .chunks_exact(4)
                .map(|c| rgb::Rgba {
                    r: c[0],
                    g: c[1],
                    b: c[2],
                    a: c[3],
                })
                .collect();
            PixelData::Rgba8(imgref::ImgVec::new(
                rgba_pixels,
                width as usize,
                height as usize,
            ))
        }
        PixelFormat::Rgb => {
            let rgb_pixels: Vec<rgb::Rgb<u8>> = buf
                .chunks_exact(3)
                .map(|c| rgb::Rgb {
                    r: c[0],
                    g: c[1],
                    b: c[2],
                })
                .collect();
            PixelData::Rgb8(imgref::ImgVec::new(
                rgb_pixels,
                width as usize,
                height as usize,
            ))
        }
        PixelFormat::Graya => {
            // Convert Gray+Alpha to RGBA
            let rgba_pixels: Vec<rgb::Rgba<u8>> = buf
                .chunks_exact(2)
                .map(|c| rgb::Rgba {
                    r: c[0],
                    g: c[0],
                    b: c[0],
                    a: c[1],
                })
                .collect();
            PixelData::Rgba8(imgref::ImgVec::new(
                rgba_pixels,
                width as usize,
                height as usize,
            ))
        }
        PixelFormat::Gray => {
            let gray_pixels: Vec<rgb::Gray<u8>> = buf
                .into_iter()
                .map(|v| rgb::Gray::new(v))
                .collect();
            PixelData::Gray8(imgref::ImgVec::new(
                gray_pixels,
                width as usize,
                height as usize,
            ))
        }
        PixelFormat::Cmyk | PixelFormat::Cmyka => {
            // CMYK is not supported by our pixel data types; convert to RGB
            // by dropping the K channel (lossy but functional)
            return Err(CodecError::UnsupportedOperation {
                format: ImageFormat::Jxl,
                detail: "CMYK pixel format not supported",
            });
        }
    };

    Ok(DecodeOutput {
        pixels,
        info,
        #[cfg(feature = "jpeg")]
        jpeg_extras: None,
    })
}
