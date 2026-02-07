# zencodecs

Unified image codec abstraction for Rust.

Thin dispatch layer over format-specific encoders and decoders:
[zenjpeg](https://github.com/imazen/zenjpeg),
[zenwebp](https://github.com/imazen/zenwebp),
[zengif](https://github.com/imazen/zengif),
[zenavif](https://github.com/imazen/zenavif),
[png](https://crates.io/crates/png),
and [ravif](https://crates.io/crates/ravif).

## Usage

```rust
use zencodecs::{ImageFormat, DecodeRequest, EncodeRequest, PixelLayout};

// Detect format and decode
let data: &[u8] = todo!(); // your image bytes
let decoded = DecodeRequest::new(data)
    .with_output_layout(PixelLayout::Rgba8)
    .decode()?;

// Re-encode as WebP
let webp_bytes = EncodeRequest::new(ImageFormat::WebP)
    .with_quality(85.0)
    .encode(&decoded.pixels, decoded.width, decoded.height, PixelLayout::Rgba8)?;
```

## Features

Every codec is feature-gated. Enable only what you need:

```toml
[dependencies]
zencodecs = { version = "0.1", default-features = false, features = ["jpeg", "webp"] }
```

| Feature | Codec | Decode | Encode |
|---------|-------|--------|--------|
| `jpeg` | zenjpeg | Yes | Yes |
| `webp` | zenwebp | Yes | Yes |
| `gif` | zengif | Yes | Yes |
| `png` | png | Yes | Yes |
| `avif-decode` | zenavif | Yes | No |
| `avif-encode` | ravif | No | Yes |

Default features: `jpeg`, `webp`, `gif`, `png`, `avif-decode`.

## What This Crate Does

- Format detection from magic bytes or file extension
- Image metadata probing without full decode
- Decode any supported format to RGBA8/BGRA8 pixels
- Encode pixels to any supported format
- Unified Limits, cancellation, and error handling

## What This Crate Does NOT Do

- Image processing (resize, crop, sharpen) — use `zenimage`
- Color management (ICC profile application)
- Animation (first frame only for animated formats)
- Streaming decode/encode (use format-specific crates directly)

## License

MIT OR Apache-2.0
