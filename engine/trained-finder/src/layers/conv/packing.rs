//! The existing kernel's weight layout, prepared in fixed bands.

use super::{BLOCK, Conv3x3};
use shapes::errors::{EngineError, ErrorCode};

/// Pending convolution weights. Each advance packs at most 65,536 values.
pub struct ConvPacking {
    weights: Vec<f32>,
    convolution: Conv3x3,
}

impl ConvPacking {
    /// Takes weights in output, input, tap order without packing them yet.
    pub fn new(
        weights: Vec<f32>,
        bias: Vec<f32>,
        inputs: usize,
        outputs: usize,
        stride: usize,
        relu: bool,
    ) -> Result<Self, EngineError> {
        if !outputs.is_multiple_of(BLOCK)
            || !(stride == 1 || stride == 2)
            || bias.len() != outputs
            || Some(weights.len()) != outputs.checked_mul(inputs).and_then(|n| n.checked_mul(9))
        {
            return Err(EngineError::new(
                ErrorCode::EngineLoadFailed,
                "a convolution's weights do not fit its shape",
            ));
        }
        let mut packed = Vec::new();
        packed.try_reserve_exact(weights.len()).map_err(|_| {
            EngineError::new(
                ErrorCode::EngineOutOfMemory,
                "not enough memory for the side weights",
            )
        })?;
        Ok(Self {
            weights,
            convolution: Conv3x3 {
                inputs,
                outputs,
                stride,
                relu,
                packed,
                bias,
            },
        })
    }

    /// Packs one fixed band. True means all weights have been packed.
    pub fn advance(&mut self) -> Result<bool, EngineError> {
        let inputs = self.convolution.inputs;
        let end = self
            .weights
            .len()
            .min(self.convolution.packed.len().saturating_add(65_536));
        for index in self.convolution.packed.len()..end {
            let lane = index % BLOCK;
            let tap = (index / BLOCK) % 9;
            let input = (index / (BLOCK * 9)) % inputs;
            let block = index / (BLOCK * 9 * inputs);
            self.convolution
                .packed
                .push(self.weights[((block * BLOCK + lane) * inputs + input) * 9 + tap]);
        }
        Ok(end == self.weights.len())
    }

    /// Returns the existing convolution only after the last band.
    pub fn finish(self) -> Result<Conv3x3, EngineError> {
        if self.convolution.packed.len() != self.weights.len() {
            return Err(EngineError::new(
                ErrorCode::EngineOutOfSync,
                "the convolution has not finished packing",
            ));
        }
        Ok(self.convolution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::Grid;

    #[test]
    fn incremental_packing_keeps_the_existing_layout_and_sums() {
        for (inputs, outputs, stride, relu) in
            [(1, 8, 1, false), (3, 16, 2, true), (160, 160, 1, true)]
        {
            let mut numbers = crate::layers::seeded::Numbers::new(123);
            let weights: Vec<_> = (0..inputs * outputs * 9)
                .map(|_| numbers.between(-0.01, 0.01))
                .collect();
            let bias: Vec<_> = (0..outputs).map(|_| numbers.between(-1.0, 1.0)).collect();
            let old = Conv3x3::new(&weights, bias.clone(), inputs, outputs, stride, relu).unwrap();
            let mut pending =
                ConvPacking::new(weights.clone(), bias, inputs, outputs, stride, relu).unwrap();
            let mut count = 0;
            loop {
                let before = pending.convolution.packed.len();
                let done = pending.advance().unwrap();
                assert!(pending.convolution.packed.len() - before <= 65_536);
                count += 1;
                if done {
                    break;
                }
            }
            assert_eq!(count, weights.len().div_ceil(65_536));
            let new = pending.finish().unwrap();
            assert_eq!(
                old.packed.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                new.packed.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            );
            if inputs < 10 {
                let mut image = Grid::bordered(inputs, 4, 6).unwrap();
                for c in 0..inputs {
                    for y in 0..4 {
                        for x in 0..6 {
                            image.row_mut(c, y)[x] = (c * 24 + y * 6 + x) as f32 / 20.0;
                        }
                    }
                }
                let mut a = Grid::zeros(outputs, 4 / stride, 6 / stride).unwrap();
                let mut b = a.clone();
                old.run_all(&[&image], &mut a);
                new.run_all(&[&image], &mut b);
                for channel in 0..outputs {
                    assert_eq!(
                        a.plane(channel)
                            .iter()
                            .map(|v| v.to_bits())
                            .collect::<Vec<_>>(),
                        b.plane(channel)
                            .iter()
                            .map(|v| v.to_bits())
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
        assert!(
            ConvPacking::new(vec![0.0; 72], vec![0.0; 8], 1, 8, 1, true)
                .unwrap()
                .finish()
                .is_err()
        );
        assert!(ConvPacking::new(vec![], vec![], usize::MAX, 8, 1, true).is_err());
    }
}
