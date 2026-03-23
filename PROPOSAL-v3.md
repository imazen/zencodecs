# zencodecs v3 Proposal: Right-Sized Codec Orchestration

## Problem

zencodecs v2 tries to be everything: codec dispatch, format selection, quality mapping,
gain map extraction, depth map extraction, streaming encode/decode, animation, pixel
conversion, and metadata handling. This makes it:

1. **Hard to stabilize** — too many responsibilities changing at different rates
2. **Hard to depend on** — downstream crates get all the complexity even if they need one thing
3. **Confused about its boundary with zenpipe** — both want to own encode/decode orchestration
4. **Missing its zenode integration** — QualityIntent is currently in zenode with hardcoded
   boolean fields instead of using zencodecs' own FormatSet

## Proposed Role

**zencodecs answers: "what codec, at what quality, in what format?"**

It does NOT touch pixels. It does NOT own the pipeline. It does NOT do streaming.
Those are zenpipe's job.

### What stays in zencodecs

| Responsibility | Why it belongs here |
|---|---|
| `CodecRegistry` | Compile-time + runtime codec enable/disable — fundamental to "which codecs exist" |
| `FormatSet` | Compact bitflag set of formats — used by registry, policy, and auto-selection |
| `CodecPolicy` | Per-request codec filtering (killbits, allowlist, preferences) |
| `QualityProfile` | Named quality presets (lowest→lossless) with per-codec calibration |
| `QualityIntent` | Resolved per-codec quality + effort + lossless — the output of quality mapping |
| `select_format()` | Format auto-selection from ImageFacts + FormatSet + preferences |
| `SelectionTrace` | Audit trail for format selection decisions |
| `ImageFacts` | Source image properties driving selection (has_alpha, is_hdr, pixel_count) |
| `CodecConfig` | Per-codec config re-exports behind feature gates |
| `CodecId` | Identifies specific codec implementations for policy targeting |
| Format detection | `ImageFormat::detect()` from magic bytes (delegated to zencodec) |
| Codec config re-exports | `config::jpeg`, `config::webp`, etc. behind feature gates |

### What moves OUT of zencodecs

| Responsibility | Where it goes | Why |
|---|---|---|
| `DecodeRequest` one-shot decode | zenpipe (via bridge) | Pipeline owns pixel flow |
| `EncodeRequest` one-shot encode | zenpipe (via bridge) | Pipeline owns pixel flow |
| Streaming encode/decode | zenpipe | Already has Source/Sink streaming |
| Gain map extraction | zenpipe (sidecar streams) | Gain maps track through geometry ops |
| Depth map extraction | zenpipe (sidecar streams) | Same reason |
| Pixel conversion (PixelData) | Drop — use zenpixels directly | Redundant wrapper |
| Animation frame iteration | zenpipe (FrameSource already exists) | Pipeline handles frame sequencing |
| Color management (moxcms) | zenpipe (IccTransformSource exists) | Pipeline inserts CMS at right point |

### What moves INTO zencodecs

| Responsibility | From where | Why |
|---|---|---|
| `QualityIntent` zenode node | zenode/src/nodes.rs | Codec selection IS zencodecs' job |
| `Encode` zenode node (format-agnostic) | New | "Encode to io_id with these settings" |

## QualityIntent Node (redesigned for zencodecs)

Currently in zenode with hardcoded `allow_jpeg/png/gif/webp/avif/jxl` booleans.
Redesigned to use zencodecs' own `FormatSet`:

```rust
// In zencodecs, behind feature = "zenode"

/// Format selection and quality profile for encoding.
///
/// This replaces zenode's QualityIntent. Uses FormatSet (dynamic,
/// extensible) instead of hardcoded per-format booleans.
#[derive(Node, Clone, Debug)]
#[node(id = "zencodecs.quality_intent", group = Encode, phase = Encode)]
#[node(tags("quality", "auto", "format", "encode"))]
pub struct QualityIntent {
    /// Quality profile: named preset or numeric 0-100.
    #[param(default = "high")]
    #[param(section = "Main", label = "Quality Profile")]
    #[kv("qp")]
    pub profile: String,

    /// Explicit output format. Empty = auto-select.
    #[param(default = "")]
    #[param(section = "Main", label = "Output Format")]
    #[kv("format")]
    pub format: String,

    /// Device pixel ratio for quality adjustment.
    #[param(range(0.5..=10.0), default = 1.0, identity = 1.0, step = 0.5)]
    #[param(unit = "×", section = "Main")]
    #[kv("qp.dpr", "qp.dppx", "dpr", "dppx")]
    pub dpr: f32,

    /// Global lossless preference.
    #[param(default = false)]
    #[param(section = "Main")]
    #[kv("lossless")]
    pub lossless: bool,

    // Format allowlist as RIAPI keys.
    // These map to FormatSet at runtime, not hardcoded struct fields.
    // The node's from_kv reads accept.* keys and builds a FormatSet.

    /// Allow WebP in format auto-selection.
    #[param(default = false)]
    #[param(section = "Allowed Formats")]
    #[kv("accept.webp")]
    pub allow_webp: bool,

    /// Allow AVIF in format auto-selection.
    #[param(default = false)]
    #[param(section = "Allowed Formats")]
    #[kv("accept.avif")]
    pub allow_avif: bool,

    /// Allow JXL in format auto-selection.
    #[param(default = false)]
    #[param(section = "Allowed Formats")]
    #[kv("accept.jxl")]
    pub allow_jxl: bool,

    /// Allow non-sRGB color profiles.
    #[param(default = false)]
    #[param(section = "Allowed Formats")]
    #[kv("accept.color_profiles")]
    pub allow_color_profiles: bool,
}

impl QualityIntent {
    /// Build a FormatSet from the allow_* fields.
    /// Web-safe baseline (jpeg/png/gif) is always included.
    pub fn to_format_set(&self) -> FormatSet {
        let mut set = FormatSet::web_safe(); // jpeg, png, gif
        if self.allow_webp { set.insert(ImageFormat::WebP); }
        if self.allow_avif { set.insert(ImageFormat::Avif); }
        if self.allow_jxl { set.insert(ImageFormat::Jxl); }
        set
    }

    /// Parse the profile string into a QualityProfile.
    pub fn to_quality_profile(&self) -> Option<QualityProfile> {
        QualityProfile::parse(&self.profile)
    }

    /// Resolve to a concrete QualityIntent (per-codec quality values).
    pub fn resolve(&self, facts: &ImageFacts, registry: &CodecRegistry) -> ResolvedIntent {
        let format_set = self.to_format_set();
        let profile = self.to_quality_profile().unwrap_or(QualityProfile::High);
        let selected_format = if self.format.is_empty() {
            select_format(facts, &format_set, registry)
        } else {
            ImageFormat::from_name(&self.format)
        };
        let quality = profile.to_intent_with_dpr(self.dpr);
        ResolvedIntent {
            format: selected_format,
            quality,
            lossless: self.lossless,
        }
    }
}
```

## Relationship Diagram

```
                    zenode (infrastructure)
                   /          |            \
                  /           |             \
    codec crates            zenpipe          zencodecs
   (zenjpeg, etc.)      (pixel pipeline)   (codec orchestration)
   Define encode     Bridge: nodes→graph    QualityIntent node
   node params       Op fusion, streaming   Format selection
                     Sidecar streams        Quality mapping
                     NodeConverter trait     CodecRegistry
                          |                 CodecPolicy
                          |                      |
                     imageflow3 (aggregator)
                     Composes both. Owns IO.
                     RIAPI + JSON API.
```

### Dependency graph

```
zencodec (traits) ← all codec crates
zenode ← all codec crates (optional), zencodecs (optional), zenpipe (optional)
zencodecs → zencodec, zenode (optional)
zenpipe → zencodec, zenode (optional), zenresize, zenlayout
imageflow3 → zencodecs, zenpipe, zenode, all codec crates
```

**zencodecs and zenpipe are peers, not layered.** Neither depends on the other.

## Migration Plan

### Phase 1: Clean up zencodecs

1. Remove `DecodeRequest`/`EncodeRequest` one-shot APIs (moved to zenpipe bridge)
2. Remove `PixelData` wrapper (use zenpixels directly)
3. Remove stubbed streaming encoder (self-referential lifetime issue)
4. Remove gain map / depth map extraction (moved to zenpipe sidecar)
5. Keep: CodecRegistry, FormatSet, QualityProfile, QualityIntent, CodecPolicy,
   select_format, CodecConfig, CodecId, SelectionTrace, ImageFacts

### Phase 2: Add zenode integration

1. Add `zenode` optional dependency
2. Move QualityIntent from zenode to zencodecs
3. Define `zencodecs.quality_intent` node using FormatSet
4. Remove `zenode.quality_intent` from zenode (breaking: bump zenode to 0.2)
5. Add `Encode` node (format-agnostic: io_id + quality_intent reference)

### Phase 3: Wire into zenpipe bridge

1. zenpipe's bridge recognizes `zencodecs.quality_intent` node
2. Passes resolved format/quality to the encoder at pipeline end
3. NodeConverter impl in zencodecs bridges quality_intent → encoder config

## What zencodecs is NOT

- NOT a pipeline (that's zenpipe)
- NOT a pixel processor (that's zenfilters/zenresize)
- NOT a codec implementation (that's zenjpeg/zenpng/etc.)
- NOT an I/O manager (that's the aggregating crate)
- NOT a format converter (that's zenpixels-convert)

It IS the codec **oracle** — you ask it "given this image and these constraints,
what format and quality should I use?" and it tells you. The actual encoding
is done by the codec crate directly, configured by the answer zencodecs gave.

## Open Questions

1. **Should zencodecs own format detection?** Currently `zencodec::ImageFormat::detect()`
   exists in the trait crate. zencodecs wraps it with registry awareness. Keep both?

2. **Should codec config re-exports stay?** They're convenient but create tight coupling.
   Alternative: callers import codec crates directly for fine-grained config.

3. **Where does `ImageInfo` (probe result) live?** Currently in zencodec (the trait crate).
   zencodecs adds `from_bytes()` convenience. This seems fine.

4. **Animation orchestration** — who sequences frames? Currently zencodecs has
   FrameSource/FrameSink. Should this move to zenpipe entirely?

5. **How does the aggregating crate compose zencodecs + zenpipe?**
   Sketch: parse RIAPI → zenode nodes → zenpipe bridge builds graph + separates
   encode nodes → resolve QualityIntent via zencodecs → configure codec →
   zenpipe executes → codec encodes.
