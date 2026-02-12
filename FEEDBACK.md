# User Feedback Log

## 2026-02-08
- User requested research on pure-Rust JPEG XL decoder crates (jxl-oxide, jxl-rs) for potential integration into zencodecs
- User requested wasm32-wasip1 binary size comparison between jxl-oxide 0.12.5 and jxl-rs 0.3.0. Result: jxl-oxide is significantly larger (~66% more code weight). Test branch: test/jxl-oxide-size

## 2026-02-08: Implement HDR/color capabilities in ravif
User requested implementing the plan to expose HDR and wide gamut capabilities through ravif's builder API.
