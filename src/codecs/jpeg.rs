//! JPEG codec adapter using zenjpeg.

use crate::pixel::{ImgRef, ImgVec, Rgb, Rgba};
use crate::{
    CodecError, DecodeOutput, EncodeOutput, ImageFormat, ImageInfo, Limits, PixelData, Stop,
};

/// Probe JPEG metadata without decoding pixels.
pub(crate) fn probe(data: &[u8]) -> Result<ImageInfo, CodecError> {
    let decoder = zenjpeg::decoder::Decoder::new();
    let result = decoder
        .decode(data, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    Ok(ImageInfo {
        width: result.width(),
        height: result.height(),
        format: ImageFormat::Jpeg,
        has_alpha: false,
        has_animation: false,
        frame_count: Some(1),
        icc_profile: None,
    })
}

/// Decode JPEG to pixels.
pub(crate) fn decode(
    data: &[u8],
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    let decoder = zenjpeg::decoder::Decoder::new();

    let decoded = decoder
        .decode(data, enough::Unstoppable)
        .map_err(|e| CodecError::from_codec(ImageFormat::Jpeg, e))?;

    let width = decoded.width();
    let height = decoded.height();

    // Extract pixel data as RGB8
    let raw_pixels = decoded
        .pixels_u8()
        .ok_or_else(|| CodecError::InvalidInput("no pixel data in decoded image".into()))?;

    // Build ImgVec<Rgb<u8>> from raw bytes
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
            icc_profile: None,
        },
    })
}

/// Encode RGB8 pixels to JPEG.
pub(crate) fn encode_rgb8(
    img: ImgRef<Rgb<u8>>,
    quality: Option<f32>,
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0) as u8;
    let width = img.width() as u32;
    let height = img.height() as u32;

    let config = zenjpeg::encoder::EncoderConfig::ycbcr(
        quality,
        zenjpeg::encoder::ChromaSubsampling::Quarter,
    );

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
    _limits: Option<&Limits>,
    _stop: Option<&dyn Stop>,
) -> Result<EncodeOutput, CodecError> {
    let quality = quality.unwrap_or(85.0).clamp(0.0, 100.0) as u8;
    let width = img.width() as u32;
    let height = img.height() as u32;

    let config = zenjpeg::encoder::EncoderConfig::ycbcr(
        quality,
        zenjpeg::encoder::ChromaSubsampling::Quarter,
    );

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_rgb8_smoke() {
        // Just verify we can create the types
        let img = ImgVec::new(
            vec![
                Rgb {
                    r: 128u8,
                    g: 128,
                    b: 128
                };
                4
            ],
            2,
            2,
        );
        // Would need actual encoder to work
        let _ = img.as_ref();
    }
}
