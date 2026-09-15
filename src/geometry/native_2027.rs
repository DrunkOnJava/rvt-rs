//! Bounded native 2027 wall curve and side-surface decoding.
//!
//! This API consumes an independently located geometry span, not an arbitrary
//! partition. Callers must establish class, live owner, and the span boundary.
//! Decoded surfaces describe the wall's uncut model; openings, joins, and final
//! boundary representation are not established by this structure. No whole-file
//! production recovery or mesh fallback is enabled by this module.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallCurveKind {
    Line,
    Arc,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeCurve {
    pub parameters: [f64; 2],
    pub origin: [f64; 3],
    pub basis_x: [f64; 3],
    pub basis_y: Option<[f64; 3]>,
    pub radius: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeSurface {
    /// u_min, v_min, u_max, v_max; cylindrical u is radians, otherwise feet.
    pub domain: [f64; 4],
    pub origin: [f64; 3],
    pub basis_x: [f64; 3],
    pub basis_y: [f64; 3],
    pub basis_z: Option<[f64; 3]>,
    pub radius: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeWallGeometry {
    pub curve: NativeCurve,
    /// Reference, first side, second side, repeated reference surface.
    pub surfaces: [NativeSurface; 4],
}

fn number(bytes: &[u8], offset: usize) -> Result<f64, &'static str> {
    let data = bytes.get(offset..offset + 8).ok_or("truncated geometry")?;
    let n = f64::from_le_bytes(data.try_into().unwrap());
    n.is_finite().then_some(n).ok_or("nonfinite geometry")
}
fn triple(bytes: &[u8], offset: usize) -> Result<[f64; 3], &'static str> {
    Ok([
        number(bytes, offset)?,
        number(bytes, offset + 8)?,
        number(bytes, offset + 16)?,
    ])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn unit(v: [f64; 3]) -> bool {
    (dot(v, v) - 1.0).abs() <= 1e-10
}
fn basis(x: [f64; 3], y: [f64; 3]) -> bool {
    unit(x) && unit(y) && dot(x, y).abs() <= 1e-10
}

/// Decode exactly a curve plus its four adjacent surface descriptions.
///
/// The input must start at the measured `04 00 08 01` curve marker and end
/// after the repeated reference surface (488 bytes for a line, 649 for an arc).
/// Lengths derive from the field grammar, not absolute offsets in a record.
/// Kind must come from an independently validated record class. Version 2027
/// is the only measured profile. All lengths, flags, numeric values, bases,
/// domains, and repeated reference surfaces are validated before success.
pub fn decode_wall_geometry_span(
    bytes: &[u8],
    version: u32,
    kind: WallCurveKind,
) -> Result<NativeWallGeometry, &'static str> {
    if version != 2027 {
        return Err("unsupported native geometry version");
    }
    let (curve_bytes, stride) = match kind {
        WallCurveKind::Line => (68, 105),
        WallCurveKind::Arc => (101, 137),
    };
    if bytes.len() != curve_bytes + 4 * stride {
        return Err("incorrect geometry span length");
    }
    if bytes[..4] != [4, 0, 8, 1] {
        return Err("unsupported curve marker");
    }
    let parameters = [number(bytes, 4)?, number(bytes, 12)?];
    if parameters[1] <= parameters[0] || !(parameters[1] - parameters[0]).is_finite() {
        return Err("invalid curve interval");
    }
    let curve = match kind {
        WallCurveKind::Line => NativeCurve {
            parameters,
            origin: triple(bytes, 20)?,
            basis_x: triple(bytes, 44)?,
            basis_y: None,
            radius: None,
        },
        WallCurveKind::Arc => {
            if bytes[100] != 0 {
                return Err("unsupported arc suffix");
            }
            let x = triple(bytes, 20)?;
            let y = triple(bytes, 44)?;
            let radius = number(bytes, 68)?;
            if !basis(x, y) || radius <= 0.0 {
                return Err("invalid arc basis or radius");
            }
            NativeCurve {
                parameters,
                origin: triple(bytes, 76)?,
                basis_x: x,
                basis_y: Some(y),
                radius: Some(radius),
            }
        }
    };
    if !unit(curve.basis_x) {
        return Err("invalid curve direction");
    }
    let mut surfaces = Vec::new();
    for i in 0..4 {
        let at = curve_bytes + i * stride;
        let domain = [
            number(bytes, at)?,
            number(bytes, at + 8)?,
            number(bytes, at + 16)?,
            number(bytes, at + 24)?,
        ];
        if bytes[at + 32] != 1 || domain[2] <= domain[0] || domain[3] <= domain[1] {
            return Err("invalid surface domain or flag");
        }
        let origin = triple(bytes, at + 33)?;
        let x = triple(bytes, at + 57)?;
        let y = triple(bytes, at + 81)?;
        if !basis(x, y) {
            return Err("invalid surface basis");
        }
        let (z, radius) = if kind == WallCurveKind::Arc {
            let z = triple(bytes, at + 105)?;
            let radius = number(bytes, at + 129)?;
            if !unit(z) || dot(x, z).abs() > 1e-10 || dot(y, z).abs() > 1e-10 || radius <= 0.0 {
                return Err("invalid cylinder basis or radius");
            }
            (Some(z), Some(radius))
        } else {
            (None, None)
        };
        surfaces.push(NativeSurface {
            domain,
            origin,
            basis_x: x,
            basis_y: y,
            basis_z: z,
            radius,
        });
    }
    if surfaces[0] != surfaces[3] || surfaces.iter().any(|s| s.domain != surfaces[0].domain) {
        return Err("inconsistent reference surfaces");
    }
    Ok(NativeWallGeometry {
        curve,
        surfaces: surfaces.try_into().unwrap(),
    })
}

/// Decode an exact 68-byte bounded line-curve span in native feet.
/// Class/owner association must be established by the caller.
pub fn decode_line_curve_span(bytes: &[u8], version: u32) -> Result<NativeCurve, &'static str> {
    if version != 2027 {
        return Err("unsupported native geometry version");
    }
    if bytes.len() != 68 || bytes[..4] != [4, 0, 8, 1] {
        return Err("incorrect line curve span");
    }
    let parameters = [number(bytes, 4)?, number(bytes, 12)?];
    let direction = triple(bytes, 44)?;
    if parameters[1] <= parameters[0]
        || !(parameters[1] - parameters[0]).is_finite()
        || !unit(direction)
    {
        return Err("invalid line interval or direction");
    }
    Ok(NativeCurve {
        parameters,
        origin: triple(bytes, 20)?,
        basis_x: direction,
        basis_y: None,
        radius: None,
    })
}

/// Decode an exact pair of horizontal floor planes and an independent repeated
/// top plane. This establishes extrusion elevations, not profile completeness.
/// The source profile, openings, and live owner remain the caller's responsibility.
pub fn decode_floor_plane_pair(
    pair: &[u8],
    repeated_top: &[u8],
    version: u32,
) -> Result<[NativeSurface; 2], &'static str> {
    if version != 2027 {
        return Err("unsupported native geometry version");
    }
    if pair.len() != 210 || repeated_top.len() != 105 || pair[..105] != *repeated_top {
        return Err("incorrect or inconsistent floor plane bounds");
    }
    let mut surfaces = Vec::new();
    for at in [0, 105] {
        let domain = [
            number(pair, at)?,
            number(pair, at + 8)?,
            number(pair, at + 16)?,
            number(pair, at + 24)?,
        ];
        let origin = triple(pair, at + 33)?;
        let x = triple(pair, at + 57)?;
        let y = triple(pair, at + 81)?;
        if pair[at + 32] != 1
            || domain[2] <= domain[0]
            || domain[3] <= domain[1]
            || !(domain[2] - domain[0]).is_finite()
            || !(domain[3] - domain[1]).is_finite()
            || !basis(x, y)
            || x[2].abs() > 1e-10
            || y[2].abs() > 1e-10
        {
            return Err("unsupported floor plane");
        }
        surfaces.push(NativeSurface {
            domain,
            origin,
            basis_x: x,
            basis_y: y,
            basis_z: None,
            radius: None,
        });
    }
    let top = &surfaces[0];
    let bottom = &surfaces[1];
    if top.domain != bottom.domain
        || top.basis_x != bottom.basis_x
        || top.basis_y != bottom.basis_y
        || top.origin[..2] != bottom.origin[..2]
        || top.origin[2] <= bottom.origin[2]
        || !(top.origin[2] - bottom.origin[2]).is_finite()
    {
        return Err("inconsistent floor extrusion planes");
    }
    Ok(surfaces.try_into().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measured_native_geometry_and_negative_boundaries() {
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/oracle-2027/wall-geometry-spans.json"
        ))
        .unwrap();
        for case in fixtures.as_array().unwrap() {
            let text = case["hex"].as_str().unwrap();
            let bytes: Vec<u8> = (0..text.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
                .collect();
            let kind = if case["kind"] == "line" {
                WallCurveKind::Line
            } else {
                WallCurveKind::Arc
            };
            let decoded = decode_wall_geometry_span(&bytes, 2027, kind).unwrap();
            if kind == WallCurveKind::Line {
                assert_eq!(
                    decode_line_curve_span(&bytes[..68], 2027).unwrap(),
                    decoded.curve
                );
            }
            let api_height = case["api_height"].as_f64().unwrap();
            assert!(
                (decoded.surfaces[0].domain[3] - decoded.surfaces[0].domain[1] - api_height).abs()
                    < 1e-10
            );
            let width = if kind == WallCurveKind::Line {
                decoded.surfaces[1]
                    .origin
                    .iter()
                    .zip(decoded.surfaces[2].origin)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt()
            } else {
                (decoded.surfaces[1].radius.unwrap() - decoded.surfaces[2].radius.unwrap()).abs()
            };
            assert!((width - case["api_width"].as_f64().unwrap()).abs() < 1e-10);
            if kind == WallCurveKind::Line {
                for (parameter, key) in decoded.curve.parameters.iter().zip(["start", "end"]) {
                    for j in 0..3 {
                        let value = decoded.curve.origin[j] + parameter * decoded.curve.basis_x[j];
                        assert!(
                            (value - case["api_curve"][key][j].as_f64().unwrap()).abs() < 1e-10
                        );
                    }
                }
            } else {
                assert!(
                    (decoded.curve.radius.unwrap()
                        - case["api_curve"]["arc"]["radius"].as_f64().unwrap())
                    .abs()
                        < 1e-10
                );
            }
            let expected = &case["expected_surfaces"];
            for (i, surface) in decoded.surfaces.iter().enumerate() {
                for (values, key) in [
                    (surface.domain.as_slice(), "bounds"),
                    (surface.origin.as_slice(), "origin"),
                    (surface.basis_x.as_slice(), "basis_x"),
                    (surface.basis_y.as_slice(), "basis_y"),
                ] {
                    for (j, value) in values.iter().enumerate() {
                        // JSON decimal parsing may differ by one ULP from the
                        // exact binary double; API comparisons use feet tolerance.
                        assert!((value - expected[i][key][j].as_f64().unwrap()).abs() < 1e-12);
                    }
                }
            }
            assert!(decode_wall_geometry_span(&bytes, 2026, kind).is_err());
            assert!(decode_wall_geometry_span(&bytes[..bytes.len() - 1], 2027, kind).is_err());
            let mut invalid = bytes.clone();
            invalid[4..12].copy_from_slice(&f64::NAN.to_le_bytes());
            assert!(decode_wall_geometry_span(&invalid, 2027, kind).is_err());
            let mut invalid = bytes.clone();
            invalid[0] = 0;
            assert!(decode_wall_geometry_span(&invalid, 2027, kind).is_err());
            let mut invalid = bytes.clone();
            let at = if kind == WallCurveKind::Line { 68 } else { 101 };
            invalid[at + 32] = 0;
            assert!(decode_wall_geometry_span(&invalid, 2027, kind).is_err());
        }
    }

    fn unhex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect()
    }
    #[test]
    fn measured_floor_planes_match_native_api_elevations() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/oracle-2027/floor-plane-spans.json"
        ))
        .unwrap();
        for case in cases.as_array().unwrap() {
            let pair = unhex(case["pair_hex"].as_str().unwrap());
            let repeated = unhex(case["repeated_top_hex"].as_str().unwrap());
            let planes = decode_floor_plane_pair(&pair, &repeated, 2027).unwrap();
            assert!((planes[0].origin[2] - case["api_max_z"].as_f64().unwrap()).abs() < 1e-10);
            assert!((planes[1].origin[2] - case["api_min_z"].as_f64().unwrap()).abs() < 1e-10);
            assert!(decode_floor_plane_pair(&pair[..209], &repeated, 2027).is_err());
            let mut wrong = repeated.clone();
            wrong[50] ^= 1;
            assert!(decode_floor_plane_pair(&pair, &wrong, 2027).is_err());
            let mut wrong = pair.clone();
            wrong[105 + 32] = 0;
            assert!(decode_floor_plane_pair(&wrong, &repeated, 2027).is_err());
        }
    }
}
