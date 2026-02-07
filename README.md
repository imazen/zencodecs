# zencodecs

Unified image codec abstraction for Rust.

Dispatch layer over format-specific encoders and decoders:
[zenjpeg](https://github.com/imazen/zenjpeg),
[zenwebp](https://github.com/imazen/zenwebp),
[zengif](https://github.com/imazen/zengif),
[zenavif](https://github.com/imazen/zenavif),
[png](https://crates.io/crates/png),
and [ravif](https://crates.io/crates/ravif).
Optional color management via [moxcms](https://crates.io/crates/moxcms).

## Usage

```rust
use zencodecs::{ImageFormat, DecodeRequest, EncodeRequest, PixelLayout};

// Detect format and decode
let data: &[u8] = todo!(); // your image bytes
let decoded = DecodeRequest::new(data)
    .with_output_layout(PixelLayout::Rgba8)
    .decode()?;

// Re-encode as WebP
let webp = EncodeRequest::new(ImageFormat::WebP)
    .with_quality(85.0)
    .encode(&decoded.pixels, decoded.width, decoded.height, PixelLayout::Rgba8)?;

// Auto-select best format
let output = EncodeRequest::auto()
    .with_quality(80.0)
    .encode(&decoded.pixels, decoded.width, decoded.height, PixelLayout::Rgba8)?;
println!("Auto-selected: {:?}", output.format);
```

### Runtime Codec Control

```rust
use zencodecs::CodecRegistry;

// Only allow JPEG and WebP for this request
let registry = CodecRegistry::none()
    .with_decode(ImageFormat::Jpeg, true)
    .with_decode(ImageFormat::WebP, true)
    .with_encode(ImageFormat::WebP, true);

let decoded = DecodeRequest::new(data)
    .with_registry(&registry)
    .decode()?;
```

### Animation

```rust
let mut decoder = DecodeRequest::new(gif_data)
    .build()?;

while let Some(frame) = decoder.next_frame()? {
    process_frame(&frame.pixels, frame.delay_ms);
}
```

### Format-Specific Config

```rust
// Use zenjpeg's native config for fine-grained control
let jpeg_config = zencodecs::jpeg::EncoderConfig::ycbcr(92, Default::default());
let output = EncodeRequest::new(ImageFormat::Jpeg)
    .with_jpeg_config(&jpeg_config)
    .encode(&pixels, w, h, PixelLayout::Rgba8)?;
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
| `moxcms` | moxcms | Color management |

Default features: `jpeg`, `webp`, `gif`, `png`, `avif-decode`.

## Capabilities

- **Format detection** from magic bytes or file extension
- **Image probing** — dimensions, alpha, animation, ICC profile without full decode
- **Runtime codec registry** — enable/disable codecs per-request
- **Streaming decode/encode** — delegates to codec when supported, buffers when not
- **Animation** — frame iteration for GIF and animated WebP
- **Auto-selection** — pick optimal encoder based on image stats and allowed formats
- **Color management** — optional sRGB conversion via moxcms
- **Format-specific configs** — re-exports codec config types for fine-grained control

## Non-Goals

- Image processing (resize, crop, sharpen) — use `zenimage`
- Compositing or blending
- Format conversion heuristics beyond simple auto-selection

## License

MIT OR Apache-2.0
