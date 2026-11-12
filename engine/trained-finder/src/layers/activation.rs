//! The squashing functions, written as PyTorch writes them.

use crate::grid::Grid;

/// `1 / (1 + e^-x)`.
pub fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + libm::expf(-value))
}

/// SiLU, `x / (1 + e^-x)`: PyTorch divides rather than multiplying by the
/// sigmoid.
pub fn silu(value: f32) -> f32 {
    value / (1.0 + libm::expf(-value))
}

/// SiLU on every value of every map, leaving any border at zero.
pub fn silu_maps(grid: &mut Grid) {
    for channel in 0..grid.channels() {
        for row in 0..grid.rows() {
            for value in grid.row_mut(channel, row) {
                *value = silu(*value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sigmoid_is_a_half_at_zero_and_symmetric() {
        assert_eq!(sigmoid(0.0), 0.5);
        for value in [0.25f32, 1.0, 3.5, 9.0] {
            assert!((sigmoid(value) + sigmoid(-value) - 1.0).abs() < 1e-6);
        }
        assert_eq!(sigmoid(-200.0), 0.0);
        assert_eq!(sigmoid(200.0), 1.0);
    }

    #[test]
    fn silu_is_the_value_times_its_sigmoid() {
        assert_eq!(silu(0.0), 0.0);
        for value in [-4.0f32, -0.5, 0.75, 6.0] {
            assert!((silu(value) - value * sigmoid(value)).abs() < 1e-6);
        }
    }

    #[test]
    fn silu_on_maps_leaves_the_border_alone() {
        let mut grid = Grid::bordered(1, 1, 2).expect("a small grid");
        grid.row_mut(0, 0).copy_from_slice(&[1.0, -1.0]);
        silu_maps(&mut grid);
        assert_eq!(grid.row(0, 0), &[silu(1.0), silu(-1.0)]);
        assert_eq!(grid.plane(0).iter().filter(|&&v| v == 0.0).count(), 10);
    }
}
