//! # zencodecs
//!
//! Unified image codec abstraction over multiple format-specific encoders and decoders.
//!
//! ## Overview
//!
//! zencodecs is a thin dispatch layer that provides:
//! - **Format detection** from magic bytes or file extensions
//! - **Codec dispatch** to format-specific implementations
//! - **Pixel format normalization** (RGBA8/BGRA8 everywhere)
//! - **Runtime codec registry** for enabling/disabling formats
//! - **Unified error handling** across all codecs
//!
//! Each codec is feature-gated. Enable only what you need:
//!
//! ```toml
//! [dependencies]
//! zencodecs = { version = "0.1", features = ["jpeg", "webp", "png"] }
//! ```
//!
//! ## Supported Formats
//!
//! - **JPEG** (`jpeg` feature) - via [zenjpeg](https://crates.io/crates/zenjpeg)
//! - **WebP** (`webp` feature) - via [zenwebp](https://crates.io/crates/zenwebp)
//! - **GIF** (`gif` feature) - via [zengif](https://crates.io/crates/zengif)
//! - **PNG** (`png` feature) - via [png](https://crates.io/crates/png)
//! - **AVIF** (`avif-decode`/`avif-encode` features) - via [zenavif](https://crates.io/crates/zenavif) and [ravif](https://crates.io/crates/ravif)
//!
//! ## Usage Examples
//!
//! ### Detect and Decode
//!
//! ```no_run
//! use zencodecs::{ImageFormat, DecodeRequest, PixelLayout};
//!
//! let data: &[u8] = &[]; // your image bytes
//!
//! // Auto-detect format
//! let format = ImageFormat::detect(data).unwrap();
//! println!("Detected: {:?}", format);
//!
//! // Decode to RGBA8
//! let decoded = DecodeRequest::new(data)
//!     .with_output_layout(PixelLayout::Rgba8)
//!     .decode()?;
//!
//! println!("{}x{} image", decoded.width, decoded.height);
//! # Ok::<(), zencodecs::CodecError>(())
//! ```
//!
//! ### Encode to Different Format
//!
//! ```no_run
//! use zencodecs::{EncodeRequest, ImageFormat, PixelLayout};
//!
//! let pixels: &[u8] = &[]; // your pixel data
//! let width = 100;
//! let height = 100;
//!
//! // Encode to WebP with quality 85
//! let webp = EncodeRequest::new(ImageFormat::WebP)
//!     .with_quality(85.0)
//!     .encode(pixels, width, height, PixelLayout::Rgba8)?;
//!
//! println!("Encoded {} bytes", webp.data.len());
//! # Ok::<(), zencodecs::CodecError>(())
//! ```
//!
//! ### Auto-Select Format
//!
//! ```no_run
//! use zencodecs::{EncodeRequest, PixelLayout};
//!
//! let pixels: &[u8] = &[]; // your pixel data
//!
//! // Let zencodecs pick the best format
//! let output = EncodeRequest::auto()
//!     .with_quality(80.0)
//!     .encode(pixels, 100, 100, PixelLayout::Rgba8)?;
//!
//! println!("Selected: {:?}", output.format);
//! # Ok::<(), zencodecs::CodecError>(())
//! ```
//!
//! ### Probe Image Metadata
//!
//! ```no_run
//! use zencodecs::ImageInfo;
//!
//! let data: &[u8] = &[]; // your image bytes
//!
//! // Get dimensions and format without decoding
//! let info = ImageInfo::from_bytes(data)?;
//! println!("{}x{} {:?}", info.width, info.height, info.format);
//! # Ok::<(), zencodecs::CodecError>(())
//! ```
//!
//! ### Control Available Codecs
//!
//! ```no_run
//! use zencodecs::{CodecRegistry, ImageFormat, DecodeRequest};
//!
//! // Only allow JPEG and PNG
//! let registry = CodecRegistry::none()
//!     .with_decode(ImageFormat::Jpeg, true)
//!     .with_decode(ImageFormat::Png, true);
//!
//! let data: &[u8] = &[]; // your image bytes
//! let decoded = DecodeRequest::new(data)
//!     .with_registry(&registry)
//!     .decode()?;
//! # Ok::<(), zencodecs::CodecError>(())
//! ```
//!
//! ## What This Crate Does NOT Do
//!
//! - **No image processing**: No resize, crop, rotate. Use `zenimage` or similar.
//! - **No color management**: No ICC profile application (yet).
//! - **No streaming**: One-shot decode/encode only (streaming planned).
//! - **No animation**: First frame only for animated formats (animation planned).
//!
//! This is a thin dispatch layer, not an image processing framework.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

mod codecs;
mod decode;
mod encode;
mod error;
mod format;
mod info;
mod limits;
mod pixel;
mod registry;

// Re-exports
pub use decode::{DecodeOutput, DecodeRequest};
pub use encode::{EncodeOutput, EncodeRequest};
pub use error::CodecError;
pub use format::ImageFormat;
pub use info::ImageInfo;
pub use limits::{ImageMetadata, Limits, Stop};
pub use pixel::PixelLayout;
pub use registry::CodecRegistry;
