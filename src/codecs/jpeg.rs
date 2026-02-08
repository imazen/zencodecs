//! JPEG codec adapter using zenjpeg.

use crate::config::CodecConfig;
use crate::pixel::{ImgRef, ImgVec, Rgb, Rgba};
use crate::{
    CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, ImageMetadata, Limits,
    PixelData, Stop,
};

/// Probe JPEG metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let decoder = zenjpeg::decoder::Decoder::new();
    let info = decoder
        .read_info(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    Ok(ImageInfo {
        width: info.dimensions.width,
        height: info.dimensions.height,
        format: ImageFormat::Jpeg,
        has_alpha: false,
        has_animation: false,
        frame_count: Some(1),
        icc_profile: info.icc_profile,
        exif: info.exif,
        xmp: info.xmp.map(|s| s.into_bytes()),
    })
}

/// Build a zenjpeg Decoder from codec config and limits.
fn build_decoder(
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
) -> zenjpeg::decoder::Decoder {
    let mut decoder = codec_config
        .and_then(|c| c.jpeg_decoder.as_ref())
        .map(|d| d.as_ref().clone())
        .unwrap_or_default();

    if let Some(lim) = limits {
        if let Some(max_px) = lim.max_pixels {
            decoder = decoder.max_pixels(max_px);
        }
        if let Some(max_mem) = lim.max_memory_bytes {
            decoder = decoder.max_memory(max_mem);
        }
    }

    decoder
}

/// Decode JPEG to pixels.
pub(crate) fn decode(
    data: &[u8],
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let stop = crate::limits::stop_or_default(stop);
    let decoder = build_decoder(codec_config, limits);

    let mut result = decoder
        .decode(data, stop)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    let width = result.width();
    let height = result.height();

    // Validate dimensions against zencodecs limits (zenjpeg has its own, but
    // this catches width/height limits that zenjpeg doesn't check)
    if let Some(lim) = limits {
        lim.validate(width, height, 3)?;
    }

    let extras = result.extras();
    let icc_profile = extras.and_then(|e| e.icc_profile()).map(|p: &[u8]| p.to_vec());
    let exif = extras.and_then(|e| e.exif()).map(|p: &[u8]| p.to_vec());
    let xmp = extras
        .and_then(|e| e.xmp())
        .map(|s: &str| s.as_bytes().to_vec());

    let raw_pixels = result
        .pixels_u8()
        .ok_or_else(|| CodecError::InvalidInput("no pixel data in decoded image".into()))?;

    let rgb_pixels: &[Rgb<u8>] = bytemuck::cast_slice(raw_pixels);
    let img = ImgVec::new(rgb_pixels.to_vec(), width as usize, height as usize);

    let jpeg_extras = result.take_extras().map(alloc::boxed::Box::new);

    Ok(DecodeOutput {
        pixels: PixelData::Rgb8(img),
        info: ImageInfo {
            width,
            height,
            format: ImageFormat::Jpeg,
            has_alpha: false,
            has_animation: false,
            frame_count: Some(1),
            icc_profile,
            exif,
            xmp,
        },
        jpeg_extras,
    })
}

/// Encode RGB8 pixels to JPEG.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    // If user provided a full JPEG encoder config, use it
    let config = if let Some(cfg) = codec_config.and_then(|c| c.jpeg_encoder.as_ref()) {
        cfg.as_ref().clone()
    } else {
        let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0) as u8;
        zenjpeg::encoder::EncoderConfig::ycbcr(
            quality,
            zenjpeg::encoder::ChromaSubsampling::Quarter,
        )
    };

    let width = img.width() as u32;
    let height = img.height() as u32;
    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());

    let mut request = config.request();
    request = apply_metadata(request, metadata);
    if let Some(s) = stop {
        request = request.stop(s);
    }

    let jpeg_data = request
        .encode_bytes(bytes, width, height, zenjpeg::encoder::PixelLayout::Rgb8Srgb)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    Ok(EncodeOutput {
        data: jpeg_data,
        format: ImageFormat::Jpeg,
    })
}

/// Encode RGBA8 pixels to JPEG (alpha is discarded).
pub(crate) fn encode_rgba8(
    img: ImgRef<Rgba<u8>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let config = if let Some(cfg) = codec_config.and_then(|c| c.jpeg_encoder.as_ref()) {
        cfg.as_ref().clone()
    } else {
        let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0) as u8;
        zenjpeg::encoder::EncoderConfig::ycbcr(
            quality,
            zenjpeg::encoder::ChromaSubsampling::Quarter,
        )
    };

    let width = img.width() as u32;
    let height = img.height() as u32;
    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());

    let mut request = config.request();
    request = apply_metadata(request, metadata);
    if let Some(s) = stop {
        request = request.stop(s);
    }

    let jpeg_data = request
        .encode_bytes(bytes, width, height, zenjpeg::encoder::PixelLayout::Rgba8Srgb)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    Ok(EncodeOutput {
        data: jpeg_data,
        format: ImageFormat::Jpeg,
    })
}

/// Apply zencodecs ImageMetadata to a zenjpeg EncodeRequest.
fn apply_metadata<'a>(
    mut request: zenjpeg::encoder::EncodeRequest<'a>,
    metadata: Option<&'a ImageMetadata<'a>>,
) -> zenjpeg::encoder::EncodeRequest<'a> {
    if let Some(meta) = metadata {
        if let Some(icc) = meta.icc_profile {
            request = request.icc_profile(icc);
        }
        if let Some(exif) = meta.exif {
            request = request.exif(zenjpeg::encoder::Exif::raw(exif.to_vec()));
        }
        if let Some(xmp) = meta.xmp {
            request = request.xmp(xmp);
        }
    }
    request
}
