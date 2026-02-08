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
    let result = decoder
        .decode(data, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    let icc_profile = result
        .extras()
        .and_then(|e| e.icc_profile())
        .map(|p| p.to_vec());

    Ok(ImageInfo {
        width: result.width(),
        height: result.height(),
        format: ImageFormat::Jpeg,
        has_alpha: false,
        has_animation: false,
        frame_count: Some(1),
        icc_profile,
    })
}

/// Decode JPEG to pixels.
pub(crate) fn decode(
    data: &[u8],
    _codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    // TODO: use codec_config.jpeg_decoder when zenjpeg decoder supports DecodeConfig
    let decoder = zenjpeg::decoder::Decoder::new();

    let decoded = decoder
        .decode(data, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    let width = decoded.width();
    let height = decoded.height();

    let icc_profile = decoded
        .extras()
        .and_then(|e| e.icc_profile())
        .map(|p| p.to_vec());

    let raw_pixels = decoded
        .pixels_u8()
        .ok_or_else(|| CodecError::InvalidInput("no pixel data in decoded image".into()))?;

    let rgb_pixels: &[Rgb<u8>] = bytemuck::cast_slice(raw_pixels);
    let img = ImgVec::new(rgb_pixels.to_vec(), width as usize, height as usize);

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
        },
    })
}

/// Encode RGB8 pixels to JPEG.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    quality: Option<f32>,
    metadata: Option<&ImageMetadata<'_>>,
    codec_config: Option<&CodecConfig>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    // If user provided a full JPEG encoder config, use it
    if let Some(config) = codec_config.and_then(|c| c.jpeg_encoder.as_ref()) {
        return encode_with_config(config, img, metadata);
    }

    let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0) as u8;
    let width = img.width() as u32;
    let height = img.height() as u32;

    let mut config = zenjpeg::encoder::EncoderConfig::ycbcr(
        quality,
        zenjpeg::encoder::ChromaSubsampling::Quarter,
    );

    // Apply metadata to config
    if let Some(meta) = metadata {
        if let Some(icc) = meta.icc_profile {
            config = config.icc_profile(icc.to_vec());
        }
        if let Some(exif) = meta.exif {
            config = config.exif(zenjpeg::encoder::Exif::Raw(exif.to_vec()));
        }
        if let Some(xmp) = meta.xmp {
            config = config.xmp(xmp.to_vec());
        }
    }

    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());

    let mut encoder = config
        .encode_from_bytes(width, height, zenjpeg::encoder::PixelLayout::Rgb8Srgb)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    encoder
        .push_packed(bytes, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    let jpeg_data = encoder
        .finish()
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
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0) as u8;
    let width = img.width() as u32;
    let height = img.height() as u32;

    let mut config = if let Some(cfg) = codec_config.and_then(|c| c.jpeg_encoder.as_ref()) {
        cfg.as_ref().clone()
    } else {
        zenjpeg::encoder::EncoderConfig::ycbcr(
            quality,
            zenjpeg::encoder::ChromaSubsampling::Quarter,
        )
    };

    if let Some(meta) = metadata {
        if let Some(icc) = meta.icc_profile {
            config = config.icc_profile(icc.to_vec());
        }
        if let Some(exif) = meta.exif {
            config = config.exif(zenjpeg::encoder::Exif::Raw(exif.to_vec()));
        }
        if let Some(xmp) = meta.xmp {
            config = config.xmp(xmp.to_vec());
        }
    }

    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());

    let mut encoder = config
        .encode_from_bytes(width, height, zenjpeg::encoder::PixelLayout::Rgba8Srgb)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    encoder
        .push_packed(bytes, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    let jpeg_data = encoder
        .finish()
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    Ok(EncodeOutput {
        data: jpeg_data,
        format: ImageFormat::Jpeg,
    })
}

/// Encode using a fully-specified JPEG encoder config (RGB8 path).
fn encode_with_config(
    config: &zenjpeg::encoder::EncoderConfig,
    img: ImgRef<Rgb<u8>>,
    metadata: Option<&ImageMetadata<'_>>,
) -> Result<EncodeOutput, CodecError> {
    let width = img.width() as u32;
    let height = img.height() as u32;

    let mut config = config.clone();

    if let Some(meta) = metadata {
        if let Some(icc) = meta.icc_profile {
            config = config.icc_profile(icc.to_vec());
        }
        if let Some(exif) = meta.exif {
            config = config.exif(zenjpeg::encoder::Exif::Raw(exif.to_vec()));
        }
        if let Some(xmp) = meta.xmp {
            config = config.xmp(xmp.to_vec());
        }
    }

    let (buf, _, _) = img.to_contiguous_buf();
    let bytes: &[u8] = bytemuck::cast_slice(buf.as_ref());

    let mut encoder = config
        .encode_from_bytes(width, height, zenjpeg::encoder::PixelLayout::Rgb8Srgb)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    encoder
        .push_packed(bytes, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    let jpeg_data = encoder
        .finish()
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    Ok(EncodeOutput {
        data: jpeg_data,
        format: ImageFormat::Jpeg,
    })
}
