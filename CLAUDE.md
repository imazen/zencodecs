# zencodecs

Unified image codec abstraction over zenjpeg, zenwebp, zengif, zenavif, png, and ravif.

See `/home/lilith/work/codec-design/README.md` for API design guidelines.
See `/home/lilith/work/zendiff/API_COMPARISON.md` for per-codec convergence status.

## Purpose

A thin dispatch layer that lets callers encode/decode images without knowing which
format-specific crate to call. Used by image proxies, CLI tools, and batch processors
that handle multiple formats.

**This is NOT a kitchen-sink image library.** It does format detection, codec dispatch,
and pixel format normalization. It does NOT do resizing, color management, compositing,
or any image processing. Those belong in higher-level crates.

## Design Rules

**No backwards compatibility required** — we have no external users. Just bump the 0.x major version for breaking changes. No deprecation shims or legacy aliases — delete old APIs. Prefer one obvious way to do things — no duplicate entry points. Minimize API surface for forwards compatibility. Avoid free functions — use methods on types (Config, Request, Decoder) instead.

**Builder convention**: `with_` prefix for consuming builder setters, bare-name for getters.

**Project standards**: `#![forbid(unsafe_code)]` with default features. no_std+alloc (minimum: wasm32). CI with codecov. README with badges and usage examples. As of Rust 1.92, almost everything is in `core::` (including `Error`) — don't assume `std` is needed. Use `wasmtimer` crate for timing on wasm. Fuzz targets required (decode, roundtrip, limits, streaming). Codecs must be safe for malicious input on real-time image proxies — no amplification, bound memory/CPU, periodic DoS/security audits.

## Architecture

```
zencodecs/
├── src/
│   ├── lib.rs          # Public API, re-exports
│   ├── format.rs       # ImageFormat enum, detection (magic bytes)
│   ├── error.rs        # CodecError (unified error type)
│   ├── decode.rs       # DecodeRequest, DecodeOutput
│   ├── encode.rs       # EncodeRequest, EncodeOutput
│   ├── info.rs         # ImageInfo (unified probe)
│   ├── pixel.rs        # PixelLayout re-export or local definition
│   └── codecs/
│       ├── mod.rs      # Codec trait + registry
│       ├── jpeg.rs     # #[cfg(feature = "jpeg")] zenjpeg adapter
│       ├── webp.rs     # #[cfg(feature = "webp")] zenwebp adapter
│       ├── gif.rs      # #[cfg(feature = "gif")] zengif adapter
│       ├── png.rs      # #[cfg(feature = "png")] png crate adapter
│       ├── avif_dec.rs # #[cfg(feature = "avif-decode")] zenavif adapter
│       └── avif_enc.rs # #[cfg(feature = "avif-encode")] ravif adapter
├── Cargo.toml
├── CLAUDE.md
├── justfile
└── README.md
```

## Public API Spec

### Format Detection

```rust
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    Jpeg,
    WebP,
    Gif,
    Png,
    Avif,
}

impl ImageFormat {
    /// Detect format from magic bytes. Returns None if unrecognized.
    pub fn detect(data: &[u8]) -> Option<Self>;

    /// Detect format from file extension.
    pub fn from_extension(ext: &str) -> Option<Self>;

    /// MIME type string.
    pub fn mime_type(&self) -> &'static str;

    /// Common file extensions.
    pub fn extensions(&self) -> &'static [&'static str];

    /// Whether this format can be decoded with current features.
    pub fn can_decode(&self) -> bool;

    /// Whether this format can be encoded with current features.
    pub fn can_encode(&self) -> bool;
}
```

### Probing

```rust
/// Unified image metadata from header parsing.
#[derive(Clone, Debug)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub has_alpha: bool,
    pub has_animation: bool,
    pub icc_profile: Option<alloc::vec::Vec<u8>>,
    // non_exhaustive via private field or #[non_exhaustive]
}

impl ImageInfo {
    /// Probe image metadata without decoding pixels.
    /// Dispatches to the appropriate codec's probe based on detected format.
    pub fn from_bytes(data: &[u8]) -> Result<Self, CodecError>;
}
```

### Decoding

```rust
pub struct DecodeRequest<'a> {
    data: &'a [u8],
    format: Option<ImageFormat>,  // None = auto-detect
    output_layout: PixelLayout,   // default: Rgba8
    limits: Option<&'a Limits>,
    stop: Option<&'a dyn Stop>,
}

impl<'a> DecodeRequest<'a> {
    pub fn new(data: &'a [u8]) -> Self;

    /// Override auto-detection (e.g., caller already knows the format).
    pub fn with_format(self, format: ImageFormat) -> Self;

    pub fn with_output_layout(self, layout: PixelLayout) -> Self;
    pub fn with_limits(self, limits: &'a Limits) -> Self;
    pub fn with_stop(self, stop: &'a dyn Stop) -> Self;

    /// Decode to pixels.
    pub fn decode(self) -> Result<DecodeOutput, CodecError>;

    /// Decode into a pre-allocated buffer.
    pub fn decode_into(self, output: &mut [u8]) -> Result<ImageInfo, CodecError>;
}

pub struct DecodeOutput {
    pub pixels: alloc::vec::Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub layout: PixelLayout,
    pub info: ImageInfo,
}
```

### Encoding

```rust
pub struct EncodeRequest<'a> {
    format: ImageFormat,
    quality: Option<f32>,         // 0-100, format-mapped
    effort: Option<u32>,          // speed/quality tradeoff
    lossless: bool,
    limits: Option<&'a Limits>,
    stop: Option<&'a dyn Stop>,
    metadata: Option<&'a ImageMetadata<'a>>,
}

impl<'a> EncodeRequest<'a> {
    pub fn new(format: ImageFormat) -> Self;

    /// Quality 0-100. Mapped to each codec's native scale.
    /// Not all formats support this (GIF, lossless PNG ignore it).
    pub fn with_quality(self, quality: f32) -> Self;

    /// Speed/quality tradeoff. Higher = faster, lower quality.
    pub fn with_effort(self, effort: u32) -> Self;

    /// Request lossless encoding (formats that support it).
    pub fn with_lossless(self, lossless: bool) -> Self;

    pub fn with_limits(self, limits: &'a Limits) -> Self;
    pub fn with_stop(self, stop: &'a dyn Stop) -> Self;
    pub fn with_metadata(self, meta: &'a ImageMetadata<'a>) -> Self;

    /// Encode pixels to the target format.
    pub fn encode(
        self,
        pixels: &[u8],
        width: u32,
        height: u32,
        layout: PixelLayout,
    ) -> Result<alloc::vec::Vec<u8>, CodecError>;

    /// Encode into a pre-allocated buffer.
    pub fn encode_into(
        self,
        pixels: &[u8],
        width: u32,
        height: u32,
        layout: PixelLayout,
        output: &mut alloc::vec::Vec<u8>,
    ) -> Result<(), CodecError>;
}
```

### Shared Types

```rust
/// Re-export or define locally — must match underlying codecs.
#[non_exhaustive]
pub enum PixelLayout {
    Rgb8,
    Rgba8,
    Bgr8,
    Bgra8,
    // Only layouts ALL codecs can handle via the unified API.
    // Format-specific layouts (Yuv420, Gray8, etc.) require
    // using the codec directly.
}

#[derive(Clone, Debug, Default)]
pub struct Limits {
    pub max_width: Option<u64>,
    pub max_height: Option<u64>,
    pub max_pixels: Option<u64>,
    pub max_memory_bytes: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct ImageMetadata<'a> {
    pub icc_profile: Option<&'a [u8]>,
    pub exif: Option<&'a [u8]>,
    pub xmp: Option<&'a [u8]>,
}

/// Unified error type. Wraps format-specific errors.
#[derive(Debug)]
#[non_exhaustive]
pub enum CodecError {
    /// Format not recognized from magic bytes.
    UnrecognizedFormat,
    /// Format recognized but codec not enabled (missing feature flag).
    UnsupportedFormat(ImageFormat),
    /// Format doesn't support requested operation (e.g., encode GIF as lossless).
    UnsupportedOperation { format: ImageFormat, detail: &'static str },
    /// Input validation failed.
    InvalidInput(alloc::string::String),
    /// Resource limit exceeded.
    LimitExceeded(alloc::string::String),
    /// Operation cancelled via Stop token.
    Cancelled,
    /// Allocation failure.
    Oom,
    /// Underlying codec error.
    Codec { format: ImageFormat, source: alloc::boxed::Box<dyn core::error::Error + Send + Sync> },
}
```

## Internal Codec Adapter Pattern

Each `codecs/*.rs` file implements a thin adapter between the unified API and the
format-specific crate. The adapter is responsible for:

1. **Pixel format conversion**: Convert between zencodecs PixelLayout and the codec's
   native pixel format enum. RGBA8/BGRA8 must always work (set A=255 on decode for
   alpha-less formats, ignore A on encode).

2. **Quality mapping**: Map the 0-100 quality scale to the codec's native scale.
   Document the mapping. For formats without quality (GIF, lossless PNG), ignore it.

3. **Error wrapping**: Wrap codec-specific errors into `CodecError::Codec { format, source }`.

4. **Limits forwarding**: Pass Limits through to the underlying codec's Limits type.

5. **Cancellation forwarding**: Pass `&dyn Stop` through.

Example adapter skeleton:

```rust
// codecs/jpeg.rs
#[cfg(feature = "jpeg")]
pub(crate) fn decode_jpeg(
    data: &[u8],
    output_layout: PixelLayout,
    limits: Option<&Limits>,
    stop: Option<&dyn Stop>,
) -> Result<DecodeOutput, CodecError> {
    // 1. Map PixelLayout to zenjpeg's layout enum
    // 2. Create zenjpeg DecoderConfig + DecodeRequest
    // 3. Forward limits and stop
    // 4. Decode
    // 5. Wrap output into DecodeOutput
    todo!()
}
```

## Quality Mapping Table

| zencodecs quality | JPEG (zenjpeg) | WebP lossy (zenwebp) | AVIF (ravif) | PNG | GIF |
|-------------------|----------------|----------------------|--------------|-----|-----|
| 0-100 | 0-100 (native) | 0-100 (native) | mapped to distance | N/A (lossless) | N/A (palette) |
| lossless=true | N/A (error) | WebP lossless | N/A | Default | Default |

Quality mapping for AVIF needs care — ravif uses 0-100 quality where 100=lossless,
but the underlying metric is butteraugli distance. Document the mapping precisely.

## What This Crate Does NOT Do

- **No image processing**: No resize, crop, rotate, sharpen, blur. Use `zenimage` or similar.
- **No color management**: No ICC profile application. That's a higher-level concern.
- **No format conversion logic**: It dispatches to codecs. The caller decides source→dest.
- **No streaming API**: The unified layer is one-shot only. For streaming, use codecs directly.
- **No animation**: First frame only for animated formats. For animation, use codecs directly.

These boundaries are intentional. This crate is a thin dispatch layer, not a framework.

## Implementation Order

1. **format.rs**: Magic byte detection, format enum — no codec deps needed
2. **error.rs**: CodecError enum
3. **pixel.rs**: PixelLayout (minimal subset: Rgb8, Rgba8, Bgr8, Bgra8)
4. **info.rs**: ImageInfo + probe (can start with just magic bytes, add per-codec probe later)
5. **decode.rs**: DecodeRequest + DecodeOutput structs
6. **encode.rs**: EncodeRequest struct
7. **codecs/jpeg.rs**: First adapter (zenjpeg is closest to target API)
8. **codecs/webp.rs**: Second adapter (zenwebp also close)
9. **codecs/png.rs**: Third adapter (png crate is external, different API style)
10. **codecs/gif.rs**: Fourth adapter (zengif, animation = first frame only)
11. **codecs/avif_dec.rs**: Fifth (zenavif — decode only)
12. **codecs/avif_enc.rs**: Sixth (ravif — encode only, external crate)

## Underlying Codec Status

| Codec | Crate | API Maturity | Notes |
|-------|-------|-------------|-------|
| JPEG | zenjpeg | High | Three-layer pattern, EncodeRequest, Limits, EncodeStats |
| WebP | zenwebp | High | All API convergence complete (2026-02-06) |
| GIF | zengif | Medium | EncodeRequest migration complete, modularized |
| PNG | png 0.18 | External | Well-maintained, different API style |
| AVIF decode | zenavif | Low | Decode works, API needs convergence |
| AVIF encode | ravif 0.13 | External | Well-maintained, own API |

For zenjpeg and zenwebp, the adapter can use their native Request/Config APIs directly.
For png and ravif, the adapter wraps their existing APIs.
For zenavif and zengif, the adapter may need to handle API gaps.

## Known Issues

(none yet — crate not implemented)

## User Feedback Log

See [FEEDBACK.md](FEEDBACK.md) if it exists.
