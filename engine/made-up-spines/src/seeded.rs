//! Every random draw comes from the recipe seed, through our own generator:
//! SplitMix64 seeding xoshiro256**, one stream per purpose, named by an FNV-1a
//! hash of its tag. No random-number crate is used, so the streams never shift
//! when a dependency changes, and adding a feature never moves another
//! feature's numbers.
//!
//! - **SplitMix64** (Steele, Lea and Flood 2014, constants as in Vigna's
//!   reference code) turns one 64-bit seed into a sequence of well-mixed
//!   words.
//! - **xoshiro256\*\*** (Blackman and Vigna 2021) is seeded from four
//!   consecutive SplitMix64 words.
//! - **FNV-1a 64** names a stream: `Stream::new(tag, seed)` seeds from
//!   `seed ^ fnv1a64(tag)`.
//!
//! Every distribution below spends a fixed number of draws per value where
//! it can (the normal discards Box–Muller's second value), so one value
//! never shifts the draws after it. A stream's order of draws is part of
//! its recipe: reordering them changes every made-up spine built from it.

/// SplitMix64: a 64-bit counter passed through a mixing function.
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub const fn new(seed: u64) -> Self {
        SplitMix64 { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// FNV-1a over bytes, 64 bits: offset `0xcbf29ce484222325`, prime
/// `0x100000001b3`. Stable across Rust releases, unlike the standard
/// library's hasher.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3)
    })
}

/// The seed of one attempt at building a recipe: the recipe's own seed
/// first, then a fresh well-mixed seed for each retry, different for
/// every pair of seed and attempt.
pub fn attempt_seed(seed: u32, attempt: u32) -> u64 {
    if attempt == 0 {
        u64::from(seed)
    } else {
        SplitMix64::new((u64::from(attempt) << 32) | u64::from(seed)).next_u64()
    }
}

/// One stream of random numbers: xoshiro256\*\*.
#[derive(Clone, Debug)]
pub struct Stream {
    state: [u64; 4],
}

/// How many tries the truncated normal makes before it clamps.
const TRUNCATED_TRIES: u32 = 64;

/// Below this mean a Poisson count is drawn exactly by inversion; from it
/// up, by the normal approximation.
const POISSON_EXACT_BELOW: f64 = 30.0;

impl Stream {
    /// The stream for one purpose: `tag` names it, `seed` is the case's.
    pub fn new(tag: &str, seed: u64) -> Self {
        Self::from_splitmix(seed ^ fnv1a64(tag.as_bytes()))
    }

    /// xoshiro256\*\* seeded from the first four outputs of SplitMix64.
    pub fn from_splitmix(seed: u64) -> Self {
        let mut mix = SplitMix64::new(seed);
        Stream {
            state: [
                mix.next_u64(),
                mix.next_u64(),
                mix.next_u64(),
                mix.next_u64(),
            ],
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.state;
        let result = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    /// Uniform on `[0, 1)`: the top 53 bits of one word.
    pub fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform on `[low, high)`.
    pub fn between(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.uniform()
    }

    /// A whole number from `low` to `high`, both included.
    pub fn whole(&mut self, low: i32, high: i32) -> i32 {
        let span = f64::from(high - low + 1);
        (low + (self.uniform() * span) as i32).min(high)
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        self.uniform() < p
    }

    /// Normal by Box–Muller, `sqrt(−2 ln(1 − U1)) · cos(2π U2)`; the second
    /// value is discarded so every value costs exactly two draws.
    pub fn normal(&mut self, mean: f64, sd: f64) -> f64 {
        let (u1, u2) = (self.uniform(), self.uniform());
        let radius = (-2.0 * libm::log(1.0 - u1)).sqrt();
        mean + sd * radius * libm::cos(core::f64::consts::TAU * u2)
    }

    /// Normal restricted to `[low, high]` by rejection; after 64 misses the
    /// last draw is clamped.
    pub fn truncated_normal(&mut self, mean: f64, sd: f64, low: f64, high: f64) -> f64 {
        let mut value = mean;
        for _ in 0..TRUNCATED_TRIES {
            value = self.normal(mean, sd);
            if (low..=high).contains(&value) {
                return value;
            }
        }
        value.clamp(low, high)
    }

    /// An index chosen with probability proportional to its weight.
    pub fn pick(&mut self, weights: &[f64]) -> usize {
        let total: f64 = weights.iter().sum();
        let target = self.uniform() * total;
        let mut running = 0.0;
        for (index, weight) in weights.iter().enumerate() {
            running += weight;
            if target < running {
                return index;
            }
        }
        weights.len().saturating_sub(1)
    }

    /// A Poisson count with mean `mean`. Below a mean of 30 it is exact, by
    /// inversion with one uniform; from 30 up it is the normal
    /// approximation `round(λ + √λ·z)`, not below zero, whose skew is under
    /// 0.2 standard deviations there. A mean that is not positive gives 0.
    pub fn poisson(&mut self, mean: f64) -> f64 {
        if mean.is_nan() || mean <= 0.0 {
            return 0.0;
        }
        if mean < POISSON_EXACT_BELOW {
            let target = self.uniform();
            let mut p = libm::exp(-mean);
            let mut cumulative = p;
            let mut k = 0.0;
            while target > cumulative && k < 1000.0 {
                k += 1.0;
                p *= mean / k;
                cumulative += p;
            }
            k
        } else {
            libm::round(self.normal(mean, mean.sqrt())).max(0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The reference words below were computed with an independent Python
    // version of SplitMix64, xoshiro256** and FNV-1a written from the
    // published definitions; the SplitMix64 and FNV-1a values also match
    // the authors' own reference outputs.

    #[test]
    fn splitmix_from_zero_gives_the_reference_words() {
        let mut mix = SplitMix64::new(0);
        assert_eq!(mix.next_u64(), 0xe220_a839_7b1d_cdaf);
        assert_eq!(mix.next_u64(), 0x6e78_9e6a_a1b9_65f4);
        assert_eq!(mix.next_u64(), 0x06c4_5d18_8009_454f);
    }

    #[test]
    fn xoshiro_seeded_from_zero_gives_the_reference_words() {
        let mut stream = Stream::from_splitmix(0);
        assert_eq!(stream.next_u64(), 0x99ec_5f36_cb75_f2b4);
        assert_eq!(stream.next_u64(), 0xbf6e_1f78_4956_452a);
        assert_eq!(stream.next_u64(), 0x1a5f_849d_4933_e6e0);
    }

    #[test]
    fn the_first_uniform_from_a_known_seed_matches() {
        assert_eq!(
            Stream::from_splitmix(20_261_110).uniform(),
            0.882_842_205_316_786_3
        );
    }

    #[test]
    fn fnv_gives_the_reference_hashes() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"recipe"), 0x69e7_52ff_db3f_740f);
    }

    #[test]
    fn streams_with_different_tags_differ_and_the_same_tag_repeats() {
        let first: Vec<u64> = (0..4).map(|_| Stream::new("sizes", 7).next_u64()).collect();
        assert!(first.windows(2).all(|pair| pair[0] == pair[1]));
        assert_ne!(
            Stream::new("sizes", 7).next_u64(),
            Stream::new("recipe", 7).next_u64()
        );
        assert_ne!(
            Stream::new("sizes", 7).next_u64(),
            Stream::new("sizes", 8).next_u64()
        );
    }

    #[test]
    fn the_first_attempt_keeps_the_seed_and_retries_move_away() {
        assert_eq!(attempt_seed(42, 0), 42);
        assert_ne!(attempt_seed(42, 1), attempt_seed(42, 2));
        assert_ne!(attempt_seed(42, 1), 42);
        // No two seeds share a retry.
        assert_ne!(attempt_seed(42, 2), attempt_seed(43, 1));
    }

    #[test]
    fn uniforms_stay_in_range_and_fill_it() {
        let mut stream = Stream::new("test", 1);
        let draws: Vec<f64> = (0..100_000).map(|_| stream.uniform()).collect();
        assert!(draws.iter().all(|u| (0.0..1.0).contains(u)));
        let mean = draws.iter().sum::<f64>() / draws.len() as f64;
        // The mean of 1e5 uniforms has standard deviation 1/sqrt(12e5).
        assert!((mean - 0.5).abs() < 5.0 / (12.0e5f64).sqrt(), "{mean}");
        let mut stream = Stream::new("whole", 3);
        let mut seen = [0u32; 5];
        for _ in 0..10_000 {
            let k = stream.whole(2, 6);
            seen[(k - 2) as usize] += 1;
        }
        assert!(seen.iter().all(|&count| count > 1800), "{seen:?}");
    }

    #[test]
    fn normals_have_the_asked_mean_and_spread() {
        let mut stream = Stream::new("normal", 11);
        let n = 200_000;
        let draws: Vec<f64> = (0..n).map(|_| stream.normal(3.0, 2.0)).collect();
        let mean = draws.iter().sum::<f64>() / f64::from(n);
        let var = draws.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / f64::from(n);
        // Standard error of the mean is 2/sqrt(n) ≈ 0.0045; of the
        // variance about 4·sqrt(2/n) ≈ 0.0126.
        assert!((mean - 3.0).abs() < 0.03, "{mean}");
        assert!((var - 4.0).abs() < 0.08, "{var}");
    }

    #[test]
    fn truncated_normals_stay_inside() {
        let mut stream = Stream::new("truncated", 5);
        for _ in 0..10_000 {
            let x = stream.truncated_normal(1.0, 0.05, 0.9, 1.1);
            assert!((0.9..=1.1).contains(&x));
        }
    }

    #[test]
    fn picks_follow_their_weights() {
        let mut stream = Stream::new("pick", 2);
        let mut counts = [0u32; 3];
        for _ in 0..60_000 {
            counts[stream.pick(&[1.0, 2.0, 3.0])] += 1;
        }
        // Expected 10 000, 20 000 and 30 000, each within about 1%.
        for (count, expected) in counts.iter().zip([10_000.0, 20_000.0, 30_000.0]) {
            assert!(
                (f64::from(*count) - expected).abs() < 0.03 * expected,
                "{counts:?}"
            );
        }
    }

    #[test]
    fn poisson_counts_have_mean_and_variance_equal_to_the_mean() {
        for mean in [0.3, 3.0, 9.99, 29.0, 30.0, 300.0, 3000.0] {
            let mut stream = Stream::new("poisson", 9);
            let n = 200_000;
            let draws: Vec<f64> = (0..n).map(|_| stream.poisson(mean)).collect();
            let m = draws.iter().sum::<f64>() / f64::from(n);
            let v = draws.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / f64::from(n);
            // Five standard errors of the mean, and 3% on the variance.
            assert!(
                (m - mean).abs() < 5.0 * (mean / f64::from(n)).sqrt(),
                "mean {mean}: {m}"
            );
            assert!((v / mean - 1.0).abs() < 0.03, "mean {mean}: variance {v}");
            assert!(draws.iter().all(|&k| k >= 0.0 && k == libm::round(k)));
        }
        assert_eq!(Stream::new("poisson", 1).poisson(0.0), 0.0);
    }

    #[test]
    fn small_poisson_counts_match_the_exact_probabilities() {
        // P(0) = e^−3 ≈ 0.0498, P(3) = 4.5·e^−3 ≈ 0.2240 for a mean of 3.
        let mut stream = Stream::new("poisson-exact", 4);
        let n = 100_000;
        let mut zeros = 0;
        let mut threes = 0;
        for _ in 0..n {
            match stream.poisson(3.0) as u32 {
                0 => zeros += 1,
                3 => threes += 1,
                _ => {}
            }
        }
        let p0 = f64::from(zeros) / f64::from(n);
        let p3 = f64::from(threes) / f64::from(n);
        assert!((p0 - libm::exp(-3.0)).abs() < 0.004, "{p0}");
        assert!((p3 - 4.5 * libm::exp(-3.0)).abs() < 0.007, "{p3}");
    }
}
