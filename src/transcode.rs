//! Decode→encode bridge via [`DecodeRowSink`].
//!
//! [`TranscodeSink`] collects decoded pixels from `push_decode()` and then
//! encodes them in one shot via `EncodeRequest`. This avoids materializing
//! a typed `ImgVec` — pixels go from decode buffers straight to the encoder.
//!
//! # True streaming transcode
//!
//! For encoders that support `push_rows()` (JPEG, PNG), a zero-materialization
//! pipeline is possible but requires all objects to live in the same scope.
//! See the module-level docs on [`dyn_dispatch`](crate::dyn_dispatch) for the
//! lifetime constraints. In practice, zenimage builds this pipeline using
//! concrete codec types directly, bypassing the dyn layer.

use alloc::vec::Vec;

use zencodec::decode::{DecodeRowSink, SinkError};
use zenpixels::{PixelDescriptor, PixelSliceMut};

/// Collects decoded pixels for subsequent encoding.
///
/// Use with [`DecodeRequest::push_decode()`](crate::DecodeRequest::push_decode)
/// to collect decoded pixels, then pass to an `EncodeRequest` via
/// [`as_pixel_slice()`](TranscodeSink::as_pixel_slice).
///
/// # Example
///
/// ```rust,ignore
/// let mut sink = TranscodeSink::new();
/// DecodeRequest::new(data).push_decode(&mut sink)?;
///
/// let pixels = sink.as_pixel_slice()?;
/// let output = EncodeRequest::new(ImageFormat::Jpeg)
///     .with_quality(85.0)
///     .encode_pixels(pixels)?;
/// ```
pub struct TranscodeSink {
    buf: Vec<u8>,
    width: u32,
    height: u32,
    descriptor: PixelDescriptor,
    rows_received: u32,
}

impl TranscodeSink {
    /// Create a new empty sink.
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            width: 0,
            height: 0,
            descriptor: PixelDescriptor::RGB8_SRGB,
            rows_received: 0,
        }
    }

    /// Width of the decoded image (available after push_decode completes).
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height of the decoded image.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Pixel descriptor of the decoded image.
    pub fn descriptor(&self) -> PixelDescriptor {
        self.descriptor
    }

    /// Raw pixel data as a byte slice.
    pub fn data(&self) -> &[u8] {
        &self.buf
    }

    /// Create a `PixelSlice` for encoding.
    pub fn as_pixel_slice(&self) -> Result<zenpixels::PixelSlice<'_>, alloc::string::String> {
        let stride = self.width as usize * self.descriptor.bytes_per_pixel();
        zenpixels::PixelSlice::new(
            &self.buf,
            self.width,
            self.height,
            stride,
            self.descriptor,
        )
        .map_err(|e| alloc::format!("{e}"))
    }

    /// Consume the sink and return the raw pixel buffer.
    pub fn into_data(self) -> Vec<u8> {
        self.buf
    }
}

impl Default for TranscodeSink {
    fn default() -> Self {
        Self::new()
    }
}

impl DecodeRowSink for TranscodeSink {
    fn begin(
        &mut self,
        width: u32,
        height: u32,
        descriptor: PixelDescriptor,
    ) -> Result<(), SinkError> {
        self.width = width;
        self.height = height;
        self.descriptor = descriptor;
        self.rows_received = 0;

        let total = width as usize * height as usize * descriptor.bytes_per_pixel();
        self.buf = Vec::with_capacity(total);
        // Pre-fill so provide_next_buffer can return slices into it.
        self.buf.resize(total, 0);
        Ok(())
    }

    fn provide_next_buffer(
        &mut self,
        y: u32,
        height: u32,
        width: u32,
        descriptor: PixelDescriptor,
    ) -> Result<PixelSliceMut<'_>, SinkError> {
        let bpp = descriptor.bytes_per_pixel();
        let stride = width as usize * bpp;
        let start = y as usize * stride;
        let end = start + height as usize * stride;

        if end > self.buf.len() {
            return Err(alloc::format!(
                "strip y={y} height={height} exceeds buffer (len={})",
                self.buf.len()
            )
            .into());
        }

        self.rows_received = y + height;

        PixelSliceMut::new(&mut self.buf[start..end], width, height, stride, descriptor)
            .map_err(|e| -> SinkError { alloc::format!("pixel slice: {e}").into() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcode_sink_lifecycle() {
        let mut sink = TranscodeSink::new();
        let desc = PixelDescriptor::RGB8_SRGB;

        sink.begin(10, 24, desc).unwrap();
        assert_eq!(sink.width(), 10);
        assert_eq!(sink.height(), 24);

        // Simulate 3 strips of 8 rows each
        for strip in 0..3u32 {
            let y = strip * 8;
            let mut ps = sink.provide_next_buffer(y, 8, 10, desc).unwrap();
            for row in 0..8 {
                ps.row_mut(row).fill((strip + 1) as u8);
            }
        }

        sink.finish().unwrap();

        // Verify data
        assert_eq!(sink.data().len(), 10 * 24 * 3); // 10 wide, 24 tall, RGB8
        let pixel_slice = sink.as_pixel_slice().unwrap();
        assert_eq!(pixel_slice.width(), 10);
        assert_eq!(pixel_slice.rows(), 24);
    }

    #[test]
    fn transcode_sink_default() {
        let sink = TranscodeSink::default();
        assert_eq!(sink.width(), 0);
        assert_eq!(sink.height(), 0);
    }
}
