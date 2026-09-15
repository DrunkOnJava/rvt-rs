//! Exact one-degree-of-freedom distance solve over authored linear constraints.
//! A saved orientation selects a discrete branch; it never supplies an anchor.
use super::linear::{self, Equation};
use anyhow::{Result, ensure};

pub(super) struct Distance {
    pub first: [usize; 2],
    pub second: [usize; 2],
    pub length: f64,
}
pub(super) struct Branch {
    /// A signed linear geometric orientation predicate, e.g. an oriented leg.
    pub terms: Vec<(usize, f64)>,
    pub positive: bool,
}
#[derive(Debug)]
pub(super) struct Solution {
    pub values: Vec<f64>,
    pub linear_rank: usize,
    pub final_rank: usize,
    pub candidate_count: usize,
    pub maximum_scaled_residual: f64,
    pub distance_residual: f64,
}

pub(super) fn solve_distance(
    variables: usize,
    equations: &[Equation],
    distance: &Distance,
    branch: &Branch,
) -> Result<Solution> {
    ensure!(
        distance.length.is_finite() && distance.length > 0.,
        "distance must be positive and finite"
    );
    ensure!(
        distance
            .first
            .iter()
            .chain(&distance.second)
            .all(|i| *i < variables),
        "distance variable index outside system"
    );
    ensure!(
        !branch.terms.is_empty()
            && branch
                .terms
                .iter()
                .all(|(i, c)| *i < variables && c.is_finite()),
        "invalid nonlinear orientation predicate"
    );
    let affine = linear::affine(variables, equations)?;
    ensure!(
        affine.directions.len() == 1,
        "distance solve requires exactly one authored free degree; got {}",
        affine.directions.len()
    );
    let direction = &affine.directions[0];
    let delta: [f64; 2] = std::array::from_fn(|i| {
        affine.origin[distance.second[i]] - affine.origin[distance.first[i]]
    });
    let tangent: [f64; 2] =
        std::array::from_fn(|i| direction[distance.second[i]] - direction[distance.first[i]]);
    let dot = |a: [f64; 2], b: [f64; 2]| a[0] * b[0] + a[1] * b[1];
    let a = dot(tangent, tangent);
    let b = 2. * dot(delta, tangent);
    let c = dot(delta, delta) - distance.length * distance.length;
    ensure!(
        a.is_finite() && b.is_finite() && c.is_finite() && a > 1e-20,
        "distance does not independently constrain free variable"
    );
    let discriminant = b * b - 4. * a * c;
    ensure!(
        discriminant.is_finite() && discriminant > 1e-12 * (b * b + (4. * a * c).abs()).max(1.),
        "distance has no two nondegenerate real branches"
    );
    let q = -0.5 * (b + discriminant.sqrt().copysign(b));
    ensure!(q.is_finite() && q != 0., "degenerate distance quadratic");
    let roots = [q / a, c / q];
    ensure!(
        roots.iter().all(|v| v.is_finite())
            && (roots[0] - roots[1]).abs() > 1e-10 * (1. + roots[0].abs() + roots[1].abs()),
        "unresolved distance branch separation"
    );
    let mut candidates = Vec::new();
    for t in roots {
        let values = affine
            .origin
            .iter()
            .zip(direction)
            .map(|(v, d)| v + t * d)
            .collect::<Vec<_>>();
        ensure!(
            values.iter().all(|v| v.is_finite()),
            "nonfinite distance solution"
        );
        let orientation = branch
            .terms
            .iter()
            .map(|(i, c)| c * values[*i])
            .sum::<f64>();
        let scale = 1.
            + branch
                .terms
                .iter()
                .map(|(i, c)| (c * values[*i]).abs())
                .sum::<f64>();
        ensure!(
            orientation.is_finite() && orientation.abs() > 1e-10 * scale,
            "nonlinear orientation branch is degenerate"
        );
        if (orientation > 0.) != branch.positive {
            continue;
        }
        let maximum_scaled_residual = linear::residual(&values, equations)?;
        let actual =
            std::array::from_fn(|i| values[distance.second[i]] - values[distance.first[i]]);
        let distance_residual =
            (dot(actual, actual).sqrt() - distance.length).abs() / (1. + distance.length);
        ensure!(
            distance_residual.is_finite() && distance_residual <= 1e-9,
            "distance residual exceeds tolerance"
        );
        let jacobian = 2. * dot(actual, tangent);
        ensure!(
            jacobian.is_finite() && jacobian.abs() > 1e-10 * (1. + distance.length) * a.sqrt(),
            "distance Jacobian loses rank"
        );
        candidates.push(Solution {
            values,
            linear_rank: affine.rank,
            final_rank: affine.rank + 1,
            candidate_count: 2,
            maximum_scaled_residual,
            distance_residual,
        });
    }
    ensure!(
        candidates.len() == 1,
        "saved orientation does not identify exactly one distance branch"
    );
    Ok(candidates.remove(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn eq(terms: &[(usize, f64)], value: f64) -> Equation {
        Equation {
            terms: terms.to_vec(),
            value,
        }
    }
    fn anchored() -> Vec<Equation> {
        vec![
            eq(&[(0, 1.)], 13.),
            eq(&[(1, 1.)], -5.),
            eq(&[(3, 1.), (1, -1.)], 0.),
            eq(&[(4, 1.), (0, -1.)], 0.),
            eq(&[(2, 1.), (0, -1.)], 4.),
        ]
    }
    #[test]
    fn true_diagonal_distance_moves_height_without_projecting_on_cached_axis() {
        let d = Distance {
            first: [2, 3],
            second: [4, 5],
            length: 5.,
        };
        let branch = Branch {
            terms: vec![(5, 1.), (1, -1.)],
            positive: true,
        };
        let r = solve_distance(6, &anchored(), &d, &branch).unwrap();
        assert_eq!(r.linear_rank, 5);
        assert_eq!(r.final_rank, 6);
        assert_eq!(r.candidate_count, 2);
        assert!((r.values[5] + 2.).abs() < 1e-10);
        assert!(r.distance_residual < 1e-10);
        assert!(r.maximum_scaled_residual < 1e-10);
        let negative = Branch {
            terms: branch.terms.clone(),
            positive: false,
        };
        let mirrored = solve_distance(6, &anchored(), &d, &negative).unwrap();
        assert!((mirrored.values[5] + 8.).abs() < 1e-10);
        let d = Distance {
            length: 4f64.hypot(7.),
            ..d
        };
        let r = solve_distance(6, &anchored(), &d, &branch).unwrap();
        assert!((r.values[5] - 2.).abs() < 1e-10);
    }
    #[test]
    fn missing_anchor_impossible_domain_and_tangent_degeneracy_refused() {
        let branch = Branch {
            terms: vec![(5, 1.), (1, -1.)],
            positive: true,
        };
        let mut equations = anchored();
        equations.remove(0);
        assert!(
            solve_distance(
                6,
                &equations,
                &Distance {
                    first: [2, 3],
                    second: [4, 5],
                    length: 5.
                },
                &branch
            )
            .is_err()
        );
        for length in [3., 4.] {
            assert!(
                solve_distance(
                    6,
                    &anchored(),
                    &Distance {
                        first: [2, 3],
                        second: [4, 5],
                        length
                    },
                    &branch
                )
                .is_err()
            );
        }
    }
}
