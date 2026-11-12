//! 1x1 convolution with bias: each output value is a weighted sum of the
//! input channels at the same place.

use crate::grid::Grid;
use shapes::errors::{EngineError, ErrorCode};
use std::ops::Range;

/// One 1x1 convolution. Each output value is the bias, then each input
/// channel's weighted value added in channel order.
#[derive(Clone, Debug)]
pub struct Conv1x1 {
    inputs: usize,
    /// Output channel by input channel.
    weights: Vec<f32>,
    bias: Vec<f32>,
}

impl Conv1x1 {
    /// Weights laid out as PyTorch keeps them (output channel, input
    /// channel).
    pub fn new(weights: Vec<f32>, bias: Vec<f32>, inputs: usize) -> Result<Conv1x1, EngineError> {
        if Some(weights.len()) != bias.len().checked_mul(inputs) {
            return Err(EngineError::new(
                ErrorCode::EngineLoadFailed,
                "a 1x1 convolution's weights do not fit its shape",
            ));
        }
        Ok(Conv1x1 {
            inputs,
            weights,
            bias,
        })
    }

    pub fn outputs(&self) -> usize {
        self.bias.len()
    }

    /// Writes rows `rows` of every output channel into `out`, input row `y`
    /// going to row `y - rows.start + out_first_row` of `out`.
    pub fn run(&self, input: &Grid, rows: Range<usize>, out: &mut Grid, out_first_row: usize) {
        debug_assert_eq!(input.channels(), self.inputs);
        for (output, &bias) in self.bias.iter().enumerate() {
            let weights = &self.weights[output * self.inputs..][..self.inputs];
            for y in rows.clone() {
                let out_row = out.row_mut(output, y - rows.start + out_first_row);
                out_row.fill(bias);
                for (channel, &weight) in weights.iter().enumerate() {
                    for (dest, &value) in out_row.iter_mut().zip(input.row(channel, y)) {
                        *dest += weight * value;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::seeded::Numbers;

    #[test]
    fn each_output_is_the_bias_plus_the_weighted_channels() {
        let conv =
            Conv1x1::new(vec![1.0, 2.0, -1.0, 0.5], vec![10.0, 0.0], 2).expect("a fitting shape");
        let input = Grid::from_values(2, 1, 2, vec![1.0, 2.0, 3.0, 4.0]).expect("four values");
        let mut out = Grid::zeros(2, 1, 2).expect("a small grid");
        conv.run(&input, 0..1, &mut out, 0);
        assert_eq!(out.row(0, 0), &[17.0, 20.0]);
        assert_eq!(out.row(1, 0), &[0.5, 0.0]);
    }

    #[test]
    fn row_by_row_gives_the_same_bits_as_the_plain_loop() {
        let mut numbers = Numbers::new(11);
        let (inputs, outputs, rows, columns) = (7, 3, 5, 6);
        let weights: Vec<f32> = (0..outputs * inputs)
            .map(|_| numbers.between(-1.0, 1.0))
            .collect();
        let bias: Vec<f32> = (0..outputs).map(|_| numbers.between(-1.0, 1.0)).collect();
        let mut input = Grid::bordered(inputs, rows, columns).expect("a small grid");
        for channel in 0..inputs {
            for row in 0..rows {
                for value in input.row_mut(channel, row) {
                    *value = numbers.between(-2.0, 2.0);
                }
            }
        }
        let conv = Conv1x1::new(weights.clone(), bias.clone(), inputs).expect("a fitting shape");
        let mut out = Grid::zeros(outputs, rows, columns).expect("a small grid");
        for row in 0..rows {
            conv.run(&input, row..row + 1, &mut out, row);
        }
        for o in 0..outputs {
            for y in 0..rows {
                for x in 0..columns {
                    let mut sum = bias[o];
                    for i in 0..inputs {
                        sum += weights[o * inputs + i] * input.at(i, y, x);
                    }
                    assert_eq!(out.at(o, y, x).to_bits(), sum.to_bits());
                }
            }
        }
    }
}
