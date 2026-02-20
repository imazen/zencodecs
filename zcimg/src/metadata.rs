//! Metadata parsing for `zcimg info --metadata`.
//!
//! Parses EXIF, ICC profile headers, CICP, and XMP into human-readable
//! structures for display and JSON output.

use std::collections::BTreeMap;

use serde::Serialize;
use zencodec_types::Cicp;

// --- EXIF ---

/// Curated EXIF tags to display (covers ~95% of useful inspection cases).
///
/// Orientation is excluded — it's already shown in the main info section.
const DISPLAY_TAGS: &[(exif::Tag, &str)] = &[
    (exif::Tag::Make, "Make"),
    (exif::Tag::Model, "Model"),
    (exif::Tag::Software, "Software"),
    (exif::Tag::Artist, "Artist"),
    (exif::Tag::Copyright, "Copyright"),
    (exif::Tag::ImageDescription, "Description"),
    (exif::Tag::DateTimeOriginal, "Date/Time Original"),
    (exif::Tag::DateTime, "Date/Time"),
    (exif::Tag::DateTimeDigitized, "Date/Time Digitized"),
    (exif::Tag::ExposureTime, "Exposure Time"),
    (exif::Tag::FNumber, "F-Number"),
    (exif::Tag::PhotographicSensitivity, "ISO"),
    (exif::Tag::ExposureProgram, "Exposure Program"),
    (exif::Tag::FocalLength, "Focal Length"),
    (exif::Tag::FocalLengthIn35mmFilm, "Focal Length (35mm)"),
    (exif::Tag::LensModel, "Lens Model"),
    (exif::Tag::WhiteBalance, "White Balance"),
    (exif::Tag::Flash, "Flash"),
    (exif::Tag::MeteringMode, "Metering Mode"),
    (exif::Tag::GPSLatitude, "GPS Latitude"),
    (exif::Tag::GPSLatitudeRef, "GPS Latitude Ref"),
    (exif::Tag::GPSLongitude, "GPS Longitude"),
    (exif::Tag::GPSLongitudeRef, "GPS Longitude Ref"),
    (exif::Tag::GPSAltitude, "GPS Altitude"),
];

#[derive(Debug, Serialize)]
pub struct ParsedExif {
    pub fields: BTreeMap<String, String>,
    pub total_tags: usize,
}

/// Parse raw EXIF bytes into a curated set of human-readable fields.
///
/// Handles both raw TIFF data (starting with `II`/`MM`) and APP1 payload
/// (starting with `Exif\0\0` followed by TIFF data).
pub fn parse_exif(raw: &[u8]) -> Option<ParsedExif> {
    // Strip "Exif\0\0" APP1 prefix if present — kamadak-exif expects raw TIFF
    let tiff_data = if raw.starts_with(b"Exif\0\0") {
        raw[6..].to_vec()
    } else {
        raw.to_vec()
    };

    let reader = exif::Reader::new();
    let exif = reader.read_raw(tiff_data).ok()?;

    let total_tags = exif.fields().count();
    let mut fields = BTreeMap::new();

    for &(tag, label) in DISPLAY_TAGS {
        if let Some(field) = exif.get_field(tag, exif::In::PRIMARY) {
            let value = field.display_value().with_unit(&exif).to_string();
            if !value.is_empty() {
                fields.insert(label.to_string(), value);
            }
        }
    }

    Some(ParsedExif { fields, total_tags })
}

// --- ICC ---

#[derive(Debug, Serialize)]
pub struct ParsedIcc {
    pub description: Option<String>,
    pub version: String,
    pub profile_class: String,
    pub color_space: String,
    pub pcs: String,
    /// TRC gamma value (if a simple power-law curve).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trc_gamma: Option<f32>,
    /// TRC description (e.g. "sRGB curve", "gamma 2.2", "LUT (1024 entries)").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trc_description: Option<String>,
    /// Transfer function formula, if derivable from the TRC.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trc_formula: Option<String>,
    pub size: usize,
}

/// Parse an ICC profile header to extract key fields.
///
/// The ICC header is a well-defined 128-byte structure. We also scan the
/// tag table for a `desc` tag and `rTRC` tag.
pub fn parse_icc(raw: &[u8]) -> Option<ParsedIcc> {
    if raw.len() < 128 {
        return None;
    }

    // Version: bytes 8-11, major.minor.bugfix
    let major = raw[8];
    let minor = raw[9] >> 4;
    let bugfix = raw[9] & 0x0F;
    let version = format!("{major}.{minor}.{bugfix}");

    // Profile class: bytes 12-15 (4-char signature)
    let profile_class = decode_signature(&raw[12..16]);

    // Color space: bytes 16-19
    let color_space = decode_signature(&raw[16..20]);

    // PCS (Profile Connection Space): bytes 20-23
    let pcs = decode_signature(&raw[20..24]);

    // Try to extract description and TRC from tag table
    let description = extract_icc_tag_data(raw, b"desc").and_then(|d| parse_desc_tag(d));
    let trc = extract_icc_tag_data(raw, b"rTRC").and_then(|d| parse_trc_tag(d));
    let (trc_gamma, trc_description, trc_formula) = match trc {
        Some(trc) => (trc.gamma, Some(trc.description), trc.formula),
        None => (None, None, None),
    };

    Some(ParsedIcc {
        description,
        version,
        profile_class: icc_class_name(&profile_class),
        color_space: icc_color_space_name(&color_space),
        pcs: icc_color_space_name(&pcs),
        trc_gamma,
        trc_description,
        trc_formula,
        size: raw.len(),
    })
}

/// Decode a 4-byte ICC signature into a trimmed ASCII string.
fn decode_signature(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&bytes[..4]).trim().to_string()
}

/// Human-readable ICC profile class name.
fn icc_class_name(sig: &str) -> String {
    match sig {
        "scnr" => "Input (Scanner)".to_string(),
        "mntr" => "Display".to_string(),
        "prtr" => "Output (Printer)".to_string(),
        "link" => "Device Link".to_string(),
        "spac" => "Color Space".to_string(),
        "abst" => "Abstract".to_string(),
        "nmcl" => "Named Color".to_string(),
        other => other.to_string(),
    }
}

/// Human-readable ICC color space name.
fn icc_color_space_name(sig: &str) -> String {
    match sig {
        "XYZ" => "XYZ".to_string(),
        "Lab" => "Lab".to_string(),
        "Luv" => "Luv".to_string(),
        "YCbr" | "YCbC" => "YCbCr".to_string(),
        "Yxy" => "Yxy".to_string(),
        "RGB" => "RGB".to_string(),
        "GRAY" => "Gray".to_string(),
        "HSV" => "HSV".to_string(),
        "HLS" => "HLS".to_string(),
        "CMYK" => "CMYK".to_string(),
        "CMY" => "CMY".to_string(),
        other => other.to_string(),
    }
}

/// Extract raw tag data for a given 4-byte signature from the ICC tag table.
fn extract_icc_tag_data<'a>(raw: &'a [u8], target_sig: &[u8; 4]) -> Option<&'a [u8]> {
    if raw.len() < 132 {
        return None;
    }

    let tag_count = u32::from_be_bytes([raw[128], raw[129], raw[130], raw[131]]) as usize;

    for i in 0..tag_count {
        let base = 132 + i * 12;
        if base + 12 > raw.len() {
            break;
        }
        if &raw[base..base + 4] != target_sig {
            continue;
        }

        let offset =
            u32::from_be_bytes([raw[base + 4], raw[base + 5], raw[base + 6], raw[base + 7]])
                as usize;
        let size =
            u32::from_be_bytes([raw[base + 8], raw[base + 9], raw[base + 10], raw[base + 11]])
                as usize;

        if offset + size <= raw.len() && size >= 8 {
            return Some(&raw[offset..offset + size]);
        }
        return None;
    }

    None
}

/// Parse an ICC `desc` (textDescriptionType) or `mluc` (multiLocalizedUnicodeType) tag.
fn parse_desc_tag(tag_data: &[u8]) -> Option<String> {
    let type_sig = &tag_data[0..4];

    match type_sig {
        // v2: textDescriptionType ('desc')
        b"desc" => {
            if tag_data.len() < 12 {
                return None;
            }
            let str_len =
                u32::from_be_bytes([tag_data[8], tag_data[9], tag_data[10], tag_data[11]]) as usize;
            if str_len == 0 || 12 + str_len > tag_data.len() {
                return None;
            }
            let s = &tag_data[12..12 + str_len];
            let s = std::str::from_utf8(s).ok()?;
            Some(s.trim_end_matches('\0').to_string())
        }
        // v4: multiLocalizedUnicodeType ('mluc')
        b"mluc" => {
            if tag_data.len() < 16 {
                return None;
            }
            let num_records =
                u32::from_be_bytes([tag_data[8], tag_data[9], tag_data[10], tag_data[11]]) as usize;
            if num_records == 0 || tag_data.len() < 16 + num_records * 12 {
                return None;
            }
            let rec_base = 16;
            let str_len = u32::from_be_bytes([
                tag_data[rec_base + 4],
                tag_data[rec_base + 5],
                tag_data[rec_base + 6],
                tag_data[rec_base + 7],
            ]) as usize;
            let str_offset = u32::from_be_bytes([
                tag_data[rec_base + 8],
                tag_data[rec_base + 9],
                tag_data[rec_base + 10],
                tag_data[rec_base + 11],
            ]) as usize;

            if str_offset + str_len > tag_data.len() || str_len < 2 {
                return None;
            }

            let utf16_data = &tag_data[str_offset..str_offset + str_len];
            let chars: Vec<u16> = utf16_data
                .chunks_exact(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16(&chars)
                .ok()
                .map(|s| s.trim_end_matches('\0').to_string())
        }
        _ => None,
    }
}

/// Parsed TRC (Tone Response Curve) from an ICC profile.
struct ParsedTrc {
    /// Gamma value, if it's a simple power-law curve.
    gamma: Option<f32>,
    /// Human-readable description.
    description: String,
    /// Mathematical formula, if derivable.
    formula: Option<String>,
}

/// Parse an ICC `curv` or `para` TRC tag.
fn parse_trc_tag(tag_data: &[u8]) -> Option<ParsedTrc> {
    let type_sig = &tag_data[0..4];

    match type_sig {
        b"curv" => {
            if tag_data.len() < 12 {
                return None;
            }
            let count =
                u32::from_be_bytes([tag_data[8], tag_data[9], tag_data[10], tag_data[11]]) as usize;
            match count {
                0 => Some(ParsedTrc {
                    gamma: Some(1.0),
                    description: "Linear (gamma 1.0)".to_string(),
                    formula: Some("Y = X".to_string()),
                }),
                1 => {
                    if tag_data.len() < 14 {
                        return None;
                    }
                    // u8Fixed8Number: upper byte is integer, lower byte is fraction
                    let raw = u16::from_be_bytes([tag_data[12], tag_data[13]]);
                    let gamma = raw as f32 / 256.0;
                    let formula = if (gamma - 2.2).abs() < 0.01 {
                        "Y = X^2.2".to_string()
                    } else if (gamma - 1.8).abs() < 0.01 {
                        "Y = X^1.8".to_string()
                    } else {
                        format!("Y = X^{gamma:.2}")
                    };
                    Some(ParsedTrc {
                        gamma: Some(gamma),
                        description: format!("Gamma {gamma:.2}"),
                        formula: Some(formula),
                    })
                }
                n => Some(ParsedTrc {
                    gamma: None,
                    description: format!("LUT ({n} entries)"),
                    formula: None,
                }),
            }
        }
        b"para" => {
            if tag_data.len() < 16 {
                return None;
            }
            let func_type = u16::from_be_bytes([tag_data[8], tag_data[9]]);
            // First parameter is always the gamma exponent (s15Fixed16)
            let g_raw =
                i32::from_be_bytes([tag_data[12], tag_data[13], tag_data[14], tag_data[15]]);
            let g = g_raw as f64 / 65536.0;

            // Read additional parameters based on function type
            let read_s15f16 = |offset: usize| -> Option<f64> {
                if offset + 4 <= tag_data.len() {
                    let raw = i32::from_be_bytes([
                        tag_data[offset],
                        tag_data[offset + 1],
                        tag_data[offset + 2],
                        tag_data[offset + 3],
                    ]);
                    Some(raw as f64 / 65536.0)
                } else {
                    None
                }
            };

            match func_type {
                // Type 0: Y = X^g
                0 => Some(ParsedTrc {
                    gamma: Some(g as f32),
                    description: format!("Parametric gamma {g:.2}"),
                    formula: Some(format!("Y = X^{g:.4}")),
                }),
                // Type 3: Y = (aX+b)^g if X >= d, else Y = cX  (sRGB-like)
                3 => {
                    let a = read_s15f16(16).unwrap_or(0.0);
                    let b = read_s15f16(20).unwrap_or(0.0);
                    let c = read_s15f16(24).unwrap_or(0.0);
                    let d = read_s15f16(28).unwrap_or(0.0);

                    // Detect well-known curves
                    let (desc, formula) = if is_srgb_para(g, a, b, c, d) {
                        (
                            "sRGB curve".to_string(),
                            format!(
                                "X >= {d:.4}: Y = ({a:.6}*X + {b:.6})^{g:.4}\n\
                                 X <  {d:.4}: Y = {c:.6}*X"
                            ),
                        )
                    } else {
                        (
                            format!("Parametric (type 3, gamma {g:.2})"),
                            format!(
                                "X >= {d:.4}: Y = ({a:.6}*X + {b:.6})^{g:.4}\n\
                                 X <  {d:.4}: Y = {c:.6}*X"
                            ),
                        )
                    };
                    Some(ParsedTrc {
                        gamma: Some(g as f32),
                        description: desc,
                        formula: Some(formula),
                    })
                }
                // Type 4: Y = (aX+b)^g + e if X >= d, else Y = cX + f
                4 => {
                    let a = read_s15f16(16).unwrap_or(0.0);
                    let b = read_s15f16(20).unwrap_or(0.0);
                    let c = read_s15f16(24).unwrap_or(0.0);
                    let d = read_s15f16(28).unwrap_or(0.0);
                    let e = read_s15f16(32).unwrap_or(0.0);
                    let f = read_s15f16(36).unwrap_or(0.0);
                    Some(ParsedTrc {
                        gamma: Some(g as f32),
                        description: format!("Parametric (type 4, gamma {g:.2})"),
                        formula: Some(format!(
                            "X >= {d:.4}: Y = ({a:.6}*X + {b:.6})^{g:.4} + {e:.6}\n\
                             X <  {d:.4}: Y = {c:.6}*X + {f:.6}"
                        )),
                    })
                }
                // Type 1: Y = (aX+b)^g if X >= -b/a, else 0
                1 => {
                    let a = read_s15f16(16).unwrap_or(0.0);
                    let b = read_s15f16(20).unwrap_or(0.0);
                    Some(ParsedTrc {
                        gamma: Some(g as f32),
                        description: format!("Parametric (type 1, gamma {g:.2})"),
                        formula: Some(format!("Y = ({a:.6}*X + {b:.6})^{g:.4}")),
                    })
                }
                // Type 2: Y = (aX+b)^g + c if X >= -b/a, else c
                2 => {
                    let a = read_s15f16(16).unwrap_or(0.0);
                    let b = read_s15f16(20).unwrap_or(0.0);
                    let c = read_s15f16(24).unwrap_or(0.0);
                    Some(ParsedTrc {
                        gamma: Some(g as f32),
                        description: format!("Parametric (type 2, gamma {g:.2})"),
                        formula: Some(format!("Y = ({a:.6}*X + {b:.6})^{g:.4} + {c:.6}")),
                    })
                }
                _ => Some(ParsedTrc {
                    gamma: Some(g as f32),
                    description: format!("Parametric (type {func_type}, gamma {g:.2})"),
                    formula: None,
                }),
            }
        }
        _ => None,
    }
}

/// Check if parametric TRC type 3 parameters match sRGB.
fn is_srgb_para(g: f64, a: f64, b: f64, c: f64, d: f64) -> bool {
    // sRGB: g=2.4, a=1/1.055≈0.9479, b=0.055/1.055≈0.0521, c=1/12.92≈0.0774, d=0.04045
    // Allow some tolerance for fixed-point rounding
    (g - 2.4).abs() < 0.01
        && (a - 1.0 / 1.055).abs() < 0.005
        && (b - 0.055 / 1.055).abs() < 0.005
        && (c - 1.0 / 12.92).abs() < 0.005
        && (d - 0.04045).abs() < 0.005
}

// --- CICP ---

#[derive(Debug, Serialize)]
pub struct ParsedCicp {
    pub color_primaries: u8,
    pub color_primaries_name: String,
    pub transfer_characteristics: u8,
    pub transfer_characteristics_name: String,
    /// Transfer function formula, if known for this CICP code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_formula: Option<String>,
    pub matrix_coefficients: u8,
    pub matrix_coefficients_name: String,
    pub full_range: bool,
    pub summary: String,
}

/// Transfer function formula for a CICP transfer characteristics code.
fn cicp_transfer_formula(tc: u8) -> Option<&'static str> {
    match tc {
        1 | 6 => Some(
            "V >= 0.018: V = 1.099 * L^0.45 - 0.099\n\
             V <  0.018: V = 4.500 * L",
        ),
        4 => Some("V = L^(1/2.2)  [gamma 2.2]"),
        5 => Some("V = L^(1/2.8)  [gamma 2.8]"),
        8 => Some("V = L  [linear]"),
        13 => Some(
            "X >= 0.0031308: V = 1.055 * L^(1/2.4) - 0.055\n\
             X <  0.0031308: V = 12.92 * L",
        ),
        16 => Some(
            "PQ (SMPTE ST 2084):\n\
             V = ((c1 + c2 * Y^m1) / (1 + c3 * Y^m1))^m2\n\
             c1=0.8359, c2=18.8516, c3=18.6875, m1=0.1593, m2=78.8438\n\
             Y = L / 10000 (normalized to 10000 nits)",
        ),
        18 => Some(
            "HLG (ARIB STD-B67):\n\
             L <= 1/12: V = sqrt(3 * L)\n\
             L >  1/12: V = 0.17883 * ln(12*L - 0.28467) + 0.55991",
        ),
        _ => None,
    }
}

/// Parse CICP into human-readable form using zencodec-types name helpers.
pub fn parse_cicp(cicp: &Cicp) -> ParsedCicp {
    ParsedCicp {
        color_primaries: cicp.color_primaries,
        color_primaries_name: Cicp::color_primaries_name(cicp.color_primaries).to_string(),
        transfer_characteristics: cicp.transfer_characteristics,
        transfer_characteristics_name: Cicp::transfer_characteristics_name(
            cicp.transfer_characteristics,
        )
        .to_string(),
        transfer_formula: cicp_transfer_formula(cicp.transfer_characteristics).map(String::from),
        matrix_coefficients: cicp.matrix_coefficients,
        matrix_coefficients_name: Cicp::matrix_coefficients_name(cicp.matrix_coefficients)
            .to_string(),
        full_range: cicp.full_range,
        summary: cicp.to_string(),
    }
}

// --- XMP ---

/// Return XMP as a trimmed UTF-8 string (XMP is human-readable XML).
///
/// Strips trailing whitespace from each line (XMP is often padded with spaces).
pub fn parse_xmp(raw: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(raw).ok()?;
    let trimmed: String = s
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    // Remove trailing empty lines
    let trimmed = trimmed.trim_end();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
