# API Feedback - Codec Adapter Implementation

This document tracks API issues encountered while implementing codec adapters for zencodecs.

## zenjpeg

**Date:** 2026-02-06
**Adapter:** src/codecs/jpeg.rs
**API Issues:**

1. **Feature gate not obvious:** Had to enable "decoder" feature in Cargo.toml to access `zenjpeg::decoder` module. The compilation error didn't make this clear.

2. **Private modules:** Initial attempt used `zenjpeg::types` module which is private. Had to switch to using public `decoder::` and `encoder::` modules.

3. **Type confusion:** Decoder result has `PixelFormat` enum, but encoder expects `encoder::PixelLayout` enum. These are different types for the same concept (pixel format/layout).

4. **No ICC profile extraction:** Expected `icc_profile()` method on decoder result, but it doesn't exist. Had to return None with TODO comment.

5. **Limits/Stop not integrated:** No obvious way to pass zencodecs Limits or Stop through to the decoder/encoder. Simplified to use `Decoder::new().decode()` without configuration.

**Resolution:** Simplified implementation to use basic `Decoder::new().decode()` and `EncoderConfig::ycbcr()` APIs. Marked limits/stop/ICC support as TODOs.

## zenwebp

**Date:** 2026-02-06
**Adapter:** src/codecs/webp.rs
**API Issues:**

1. **Method name mismatch:** Expected `frame_count()` method on `WebPDemuxer`, but actual method is `num_frames()`.

2. **Redundant check:** Initially wrote `demuxer.frame_count() > 1` to check animation, but `is_animated()` method exists and is more direct.

**Resolution:** Used `num_frames()` and `is_animated()` methods. Otherwise the API matched expectations well.

## zengif

**Date:** 2026-02-06
**Adapter:** src/codecs/gif.rs
**API Issues:**

1. **Type mismatch:** `decode_gif()` returns `Vec<ComposedFrame>` where `pixels` field is `Vec<Rgba>`, not `Vec<u8>`. Expected flat byte array like other codecs.

2. **Encoder input type:** `FrameInput::new()` expects `Vec<Rgba>`, not `Vec<u8>`. Had to convert from flat RGBA bytes to structured `Rgba` values.

3. **Compilation error in zengif itself:** Fixed upstream issue where `default_buffer_frames` import wasn't feature-gated (encoder.rs:7).

**Resolution:**
- Decode: Convert `Vec<Rgba>` to flat `Vec<u8>` using `flat_map(|rgba| [rgba.r, rgba.g, rgba.b, rgba.a])`
- Encode: Convert flat bytes to `Vec<Rgba>` using `chunks_exact(4).map(|c| Rgba::new(...))`
- Fixed zengif compilation error upstream

## png (external crate)

**Date:** 2026-02-06
**Adapter:** src/codecs/png.rs
**API Issues:**

None! The png crate API worked smoothly on first try.

**Implementation notes:**
- Uses `Decoder::new(Cursor::new(data))` → `read_info()` → `next_frame(&mut buf)` pattern
- Encoder uses builder: `Encoder::new()` → `set_color()` / `set_depth()` → `write_header()` → `write_image_data()`
- Decoder automatically handles transformations (indexed → RGB, grayscale expansion)
- Requires `extern crate std` due to std::io traits
- APNG support exposed via `info.animation_control`
- ICC profile extraction works: `info.icc_profile`

**Resolution:** No issues encountered. Implementation was straightforward based on documentation examples.

## Summary

The most friction came from zenjpeg where the API has multiple layers (Config, Request, etc.) but no clear examples for simple use cases. zenwebp was smooth except for minor method name differences. zengif requires type conversions because it uses a structured `Rgba` type instead of flat byte arrays. png crate (external) worked perfectly on first try with clear API and good documentation.

**Recommendations:**
- zenjpeg: Add simple usage examples to docs, unify PixelFormat/PixelLayout types
- zenwebp: Consider aliasing `frame_count()` → `num_frames()` for consistency with other codecs
- zengif: Consider providing flat byte array convenience methods alongside `Vec<Rgba>` API
- png: No changes needed - exemplary API design
