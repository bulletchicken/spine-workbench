//! Repeatable numbers for the layer tests.

/// A small seeded generator (xorshift64*), so every run of a test sees the
/// same maps and weights.
pub(crate) struct Numbers(u64);

impl Numbers {
    pub(crate) fn new(seed: u64) -> Numbers {
        Numbers(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    /// A value from `low` up to, not including, `high`.
    pub(crate) fn between(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }
}
