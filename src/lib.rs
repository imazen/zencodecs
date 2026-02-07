//! # zencodecs
//!
//! Unified image codec abstraction over multiple format-specific encoders and decoders.
//!
//! Each codec is feature-gated. Enable only what you need:
//!
//! ```toml
//! [dependencies]
//! zencodecs = { version = "0.1", features = ["jpeg", "webp", "png"] }
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use zencodecs::{ImageFormat, DecodeRequest, EncodeRequest, PixelLayout};
//!
//! // Detect and decode
//! let data: &[u8] = &[]; // your image bytes
//! let format = ImageFormat::detect(data).unwrap();
//! let decoded = DecodeRequest::new(data)
//!     .with_output_layout(PixelLayout::Rgba8)
//!     .decode()?;
//!
//! // Encode to a different format
//! let webp = EncodeRequest::new(ImageFormat::WebP)
//!     .with_quality(85.0)
//!     .encode(&decoded.pixels, decoded.width, decoded.height, PixelLayout::Rgba8)?;
//! # Ok::<(), zencodecs::CodecError>(())
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

// TODO: implement
