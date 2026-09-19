//! Bounded linear algebra for explicitly decoded sketch equations. No saved
//! coordinate is inserted as an anchor or selected to resolve a free variable.
use anyhow::{Result, ensure};

pub(in crate::native_family_geometry) struct Equation {
    pub terms: Vec<(usize, f64)>,
    pub value: f64,
}
#[derive(Debug)]
pub(in crate::native_family_geometry) struct Solution {
    pub values: Vec<f64>,
    pub rank: usize,
    pub maximum_scaled_residual: f64,
}
#[derive(Debug)]
pub(in crate::native_family_geometry) struct AffineSolution {
    pub origin: Vec<f64>,
    pub directions: Vec<Vec<f64>>,
    pub rank: usize,
}

pub(in crate::native_family_geometry) fn affine(
    variables: usize,
    equations: &[Equation],
) -> Result<AffineSolution> {
    ensure!(
        (1..=256).contains(&variables) && equations.len() <= 4096,
        "linear sketch budget exceeded"
    );
    let mut rows = Vec::with_capacity(equations.len());
    for equation in equations {
        ensure!(
            equation.value.is_finite(),
            "nonfinite sketch equation value"
        );
        let mut row = vec![0.; variables + 1];
        for &(index, coefficient) in &equation.terms {
            ensure!(
                index < variables && coefficient.is_finite(),
                "invalid sketch equation variable"
            );
            row[index] += coefficient;
            ensure!(row[index].is_finite(), "sketch coefficient overflow");
        }
        row[variables] = equation.value;
        let scale = row[..variables]
            .iter()
            .copied()
            .map(f64::abs)
            .fold(0., f64::max);
        if scale > 0. {
            for value in &mut row {
                *value /= scale;
            }
        }
        rows.push(row);
    }
    let mut rank = 0;
    let mut pivots = Vec::new();
    for column in 0..variables {
        let Some(pivot) = (rank..rows.len())
            .max_by(|&a, &b| rows[a][column].abs().total_cmp(&rows[b][column].abs()))
        else {
            break;
        };
        if rows[pivot][column].abs() <= 1e-10 {
            continue;
        }
        rows.swap(rank, pivot);
        let value = rows[rank][column];
        for item in &mut rows[rank][column..] {
            *item /= value;
        }
        let pivot_values = rows[rank][column..].to_vec();
        for (other, row) in rows.iter_mut().enumerate() {
            if other == rank {
                continue;
            }
            let factor = row[column];
            for (item, pivot) in row[column..].iter_mut().zip(&pivot_values) {
                *item -= factor * pivot;
            }
        }
        pivots.push(column);
        rank += 1;
    }
    for row in &rows {
        ensure!(
            row.iter().all(|v| v.is_finite()),
            "nonfinite linear sketch elimination"
        );
        if row[..variables].iter().all(|v| v.abs() <= 1e-10) {
            ensure!(
                row[variables].abs() <= 1e-8,
                "inconsistent explicit sketch constraints"
            );
        }
    }
    let mut origin = vec![0.; variables];
    for (row, &column) in pivots.iter().enumerate() {
        origin[column] = rows[row][variables];
    }
    let mut directions = Vec::new();
    for free in (0..variables).filter(|column| !pivots.contains(column)) {
        let mut direction = vec![0.; variables];
        direction[free] = 1.;
        for (row, &column) in pivots.iter().enumerate() {
            direction[column] = -rows[row][free];
        }
        directions.push(direction);
    }
    Ok(AffineSolution {
        origin,
        directions,
        rank,
    })
}

pub(in crate::native_family_geometry) fn residual(
    values: &[f64],
    equations: &[Equation],
) -> Result<f64> {
    let mut maximum_scaled_residual = 0f64;
    for equation in equations {
        let lhs = equation
            .terms
            .iter()
            .map(|&(i, c)| c * values[i])
            .sum::<f64>();
        let scale = 1.
            + equation.value.abs()
            + equation
                .terms
                .iter()
                .map(|&(i, c)| (c * values[i]).abs())
                .sum::<f64>();
        let residual = (lhs - equation.value).abs() / scale;
        ensure!(
            residual.is_finite() && residual <= 1e-9,
            "linear sketch residual exceeds qualification tolerance"
        );
        maximum_scaled_residual = maximum_scaled_residual.max(residual);
    }
    Ok(maximum_scaled_residual)
}

pub(in crate::native_family_geometry) fn solve(
    variables: usize,
    equations: &[Equation],
) -> Result<Solution> {
    let affine = affine(variables, equations)?;
    let rank = affine.rank;
    ensure!(
        rank == variables,
        "underconstrained sketch: rank {rank} of {variables}; no implicit anchor allowed"
    );
    let maximum_scaled_residual = residual(&affine.origin, equations)?;
    Ok(Solution {
        values: affine.origin,
        rank,
        maximum_scaled_residual,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn equation(terms: &[(usize, f64)], value: f64) -> Equation {
        Equation {
            terms: terms.to_vec(),
            value,
        }
    }
    #[test]
    fn independent_equations_solve_translated_triangle_and_redundancy() {
        // Three distinct vertices; their authored axes, two dimensions and
        // absolute origin equations determine all six coordinates.
        let equations = vec![
            equation(&[(0, 1.)], 13.),
            equation(&[(1, 1.)], -5.),
            equation(&[(3, 1.), (1, -1.)], 0.),
            equation(&[(4, 1.), (0, -1.)], 0.),
            equation(&[(2, 1.), (0, -1.)], 7.),
            equation(&[(5, 1.), (1, -1.)], 3.),
            equation(&[(4, 2.), (0, -2.)], 0.),
        ];
        let result = solve(6, &equations).unwrap();
        assert_eq!(result.values, vec![13., -5., 20., -5., 13., -2.]);
        assert_eq!(result.rank, 6);
        assert_eq!(result.maximum_scaled_residual, 0.);
    }
    #[test]
    fn free_translation_and_conflicting_anchors_are_refused() {
        let mut equations = vec![equation(&[(0, 1.), (1, -1.)], 4.)];
        assert!(
            solve(2, &equations)
                .unwrap_err()
                .to_string()
                .contains("underconstrained")
        );
        equations.extend([equation(&[(1, 1.)], 2.), equation(&[(1, 1.)], 3.)]);
        assert!(
            solve(2, &equations)
                .unwrap_err()
                .to_string()
                .contains("inconsistent")
        );
    }
}
