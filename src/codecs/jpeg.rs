//! JPEG codec adapter using zenjpeg.
//!
//! Probe and encode use the trait interface. Decode uses native API to
//! preserve JPEG extras (DecodedExtras with DCT coefficients etc.).

use crate::config::CodecConfig;
use crate::limits::to_resource_limits;
use crate::pixel::{Bgra, ImgRef, ImgVec, Rgb, Rgba};
use crate::{
    CodecError, DecodeOutput, Decoding, DecodingJob, EncodeOutput, Encoding, EncodingJob,
    ImageFormat, ImageInfo, ImageMetadata, Limits, PixelData, Stop,
};

/// Probe JPEG metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    zenjpeg::JpegDecoding::new()
        .probe(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))
}

/// Build a zenjpeg Decoder from codec config and limits.
fn build_decoder(
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
) -> zenjpeg::decoder::DecodeConfig {
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
///
/// Uses native API to preserve JPEG extras (DecodedExtras).
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
    let icc_profile = extras
        .and_then(|e| e.icc_profile())
        .map(|p: &[u8]| p.to_vec());
    let exif = extras.and_then(|e| e.exif()).map(|p: &[u8]| p.to_vec());
    let xmp = extras
        .and_then(|e| e.xmp())
        .map(|s: &str| s.as_bytes().to_vec());

    let raw_pixels = result
        .pixels_u8()
        .ok_or_else(|| CodecError::InvalidInput("no pixel data in decoded image".into()))?;

    let rgb_pixels: &[Rgb<u8>] = bytemuck::cast_slice(raw_pixels);
    let img = ImgVec::new(rgb_pixels.to_vec(), width as usize, height as usize);

    let jpeg_extras = result.take_extras();

    let mut ii = ImageInfo::new(width, height, ImageFormat::Jpeg).with_frame_count(1);
    if let Some(icc) = icc_profile {
        ii = ii.with_icc_profile(icc);
    }
    if let Some(exif) = exif {
        ii = ii.with_exif(exif);
    }
    if let Some(xmp) = xmp {
        ii = ii.with_xmp(xmp);
    }

    let mut output = DecodeOutput::new(PixelData::Rgb8(img), ii);
    if let Some(extras) = jpeg_extras {
        output = output.with_extras(extras);
    }
    Ok(output)
}

/// Compute actual output dimensions for JPEG (applies DctScale, auto_orient).
pub(crate) fn decode_info(
    data: &[u8],
    codec_config: Option<&CodecConfig>,
) -> Result<ImageInfo, CodecError> {
    let dec = build_decoding(codec_config, None);
    dec.decode_info(data)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))
}

/// Decode JPEG directly into a caller-provided RGB8 buffer.
pub(crate) fn decode_into_rgb8(
    data: &[u8],
    dst: imgref::ImgRefMut<'_, Rgb<u8>>,
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<ImageInfo, CodecError> {
    let dec = build_decoding(codec_config, limits);
    let mut job = dec.job();
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.decode_into_rgb8(data, dst)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))
}

/// Decode JPEG directly into a caller-provided RGBA8 buffer.
pub(crate) fn decode_into_rgba8(
    data: &[u8],
    dst: imgref::ImgRefMut<'_, Rgba<u8>>,
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<ImageInfo, CodecError> {
    let dec = build_decoding(codec_config, limits);
    let mut job = dec.job();
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.decode_into_rgba8(data, dst)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))
}

/// Decode JPEG directly into a caller-provided Gray8 buffer.
pub(crate) fn decode_into_gray8(
    data: &[u8],
    dst: imgref::ImgRefMut<'_, crate::pixel::Gray<u8>>,
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<ImageInfo, CodecError> {
    let dec = build_decoding(codec_config, limits);
    let mut job = dec.job();
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.decode_into_gray8(data, dst)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))
}

/// Build a JpegDecoding from codec config and limits.
fn build_decoding(
    _codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
) -> zenjpeg::JpegDecoding {
    // Note: JpegDecoding doesn't expose inner_mut() for codec-specific config.
    // The codec_config.jpeg_decoder is used by the native decode() path above.
    // For trait-based decode_into_*/decode_info, we use the default config.
    let mut dec = zenjpeg::JpegDecoding::new();
    if let Some(lim) = limits {
        dec = dec.with_limits(&to_resource_limits(lim));
    }
    dec
}

/// Build a JpegEncoding from codec config or generic quality.
fn build_encoding(
    quality: Option<f32>,
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
) -> zenjpeg::JpegEncoding {
    let mut enc = if let Some(cfg) = codec_config.and_then(|c| c.jpeg_encoder.as_ref()) {
        let mut e = zenjpeg::JpegEncoding::new();
        *e.inner_mut() = cfg.as_ref().clone();
        e
    } else {
        let q = quality.unwrap_or(85.0).clamp(0.0, 100.0);
        zenjpeg::JpegEncoding::new().with_quality(q)
    };
    if let Some(lim) = limits {
        enc = enc.with_limits(&to_resource_limits(lim));
    }
    enc
}

/// Encode RGB8 pixels to JPEG.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, codec_config, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_rgb8(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))
}

/// Encode RGBA8 pixels to JPEG (alpha is discarded).
pub(crate) fn encode_rgba8(
    img: ImgRef<Rgba<u8>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let enc = build_encoding(quality, codec_config, limits);
    let mut job = enc.job();
    if let Some(meta) = metadata {
        job = job.with_metadata(meta);
    }
    if let Some(s) = stop {
        job = job.with_stop(s);
    }
    job.encode_rgba8(img)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))
}

/// Encode BGRA8 pixels to JPEG (native BGRA path, alpha discarded).
pub(crate) fn encode_bgra8(
    img: ImgRef<Bgra<u8>>,
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
        .encode_bytes(
            bytes,
            width,
            height,
            zenjpeg::encoder::PixelLayout::Bgra8Srgb,
        )
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    Ok(EncodeOutput::new(jpeg_data, ImageFormat::Jpeg))
}

/// Encode BGRX8 pixels to JPEG (native BGRX path, padding byte ignored).
pub(crate) fn encode_bgrx8(
    img: ImgRef<Bgra<u8>>,
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
        .encode_bytes(
            bytes,
            width,
            height,
            zenjpeg::encoder::PixelLayout::Bgrx8Srgb,
        )
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    Ok(EncodeOutput::new(jpeg_data, ImageFormat::Jpeg))
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
