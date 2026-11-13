//! A named Latin hypercube: permutation and within-stratum position use
//! separate seeded streams, never the made-up recipes' draw order.

use crate::ReadingsError;
use made_up_spines::seeded::Stream;

/// A Latin hypercube whose dimensions can be requested independently.
///
/// ```
/// use spine_readings::readings::NamedSampler;
/// let sampler = NamedSampler::new(7, 32).unwrap();
/// let u = sampler.uniform("readings-example/scale", 4).unwrap();
/// let z = sampler.normal("readings-example/offset", 4).unwrap();
/// assert!(u > 0.0 && u < 1.0 && z.is_finite());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NamedSampler {
    seed: u64,
    versions: u32,
}

impl NamedSampler {
    /// `versions` is the whole ensemble size, even for a single-row batch.
    pub fn new(seed: u64, versions: u32) -> Result<Self, ReadingsError> {
        if versions == 0 {
            return Err(invalid("versions", "must-be-positive"));
        }
        Ok(Self { seed, versions })
    }

    pub fn count(&self) -> u32 {
        self.versions
    }

    /// A uniform in `(0, 1)`, one in each of N strata for a dimension.
    /// Recreating this sampler or requesting other dimensions has no effect.
    pub fn uniform(&self, name: &str, k: u32) -> Result<f64, ReadingsError> {
        if k >= self.versions {
            return Err(invalid("version", "outside-ensemble"));
        }
        // A length prefix keeps names containing separators unambiguous.
        let prefix = format!("{}:{name}", name.len());
        let mut stream = Stream::new(&format!("versions-permutation/{prefix}"), self.seed);
        let mut permutation: Vec<u32> = (0..self.versions).collect();
        for i in (1..self.versions).rev() {
            let span = u64::from(i) + 1;
            // Rejection removes modulo bias, including for non-power-of-two N.
            let threshold = span.wrapping_neg() % span;
            let mut word = stream.next_u64();
            while word < threshold {
                word = stream.next_u64();
            }
            permutation.swap(i as usize, (word % span) as usize);
        }
        let stratum = permutation[k as usize];
        let mut place = Stream::new(&format!("versions-place/{prefix}/{k}"), self.seed);
        // Midpoints of 2^52 bins exclude both endpoints exactly in f64.
        let inside = ((place.next_u64() >> 12) as f64 + 0.5) / 4_503_599_627_370_496.0;
        let n = f64::from(self.versions);
        let mut u = (f64::from(stratum) + inside) / n;
        // Division and the later multiplication by N can round onto the
        // next boundary. Keep the computed draw inside its assigned stratum.
        u = u.clamp(f64::from_bits(1), 1.0_f64.next_down());
        while libm::floor(u * n) < f64::from(stratum) {
            u = u.next_up();
        }
        while libm::floor(u * n) > f64::from(stratum) {
            u = u.next_down();
        }
        Ok(u)
    }

    /// A standard normal, using the same dimension's uniform quantile.
    pub fn normal(&self, name: &str, k: u32) -> Result<f64, ReadingsError> {
        inverse_normal(self.uniform(name, k)?)
    }
}

/// Inverse standard-normal CDF for a finite probability strictly in `(0,1)`.
///
/// A fixed bracket and 64 bisections of libm's complementary error function
/// avoid both platform maths and cancellation in the upper tail. At −40 the
/// CDF is below the smallest positive f64. Subnormal tail probabilities
/// use the log of the Gaussian tail integral to avoid CDF underflow.
pub fn inverse_normal(probability: f64) -> Result<f64, ReadingsError> {
    if !probability.is_finite() || probability <= 0.0 || probability >= 1.0 {
        return Err(invalid("probability", "must-be-in-open-unit-interval"));
    }
    if probability == 0.5 {
        return Ok(0.0);
    }
    let target = probability.min(1.0 - probability);
    let log_target = libm::log(target);
    let mut low = -40.0;
    let mut high = 0.0;
    for _ in 0..64 {
        let middle = 0.5 * (low + high);
        if middle == low || middle == high {
            break;
        }
        let below = if target < f64::MIN_POSITIVE {
            log_lower_tail(middle) < log_target
        } else {
            0.5 * libm::erfc(-middle * core::f64::consts::FRAC_1_SQRT_2) < target
        };
        if below {
            low = middle;
        } else {
            high = middle;
        }
    }
    let lower = 0.5 * (low + high);
    Ok(if probability < 0.5 { lower } else { -lower })
}

// Repeated integration by parts gives the Gaussian tail as
// exp(-x²/2)/(sqrt(2π)*|x|) * (1 - 1/x² + 3/x⁴ - 15/x⁶ + ...).
// This branch's bisection points have |x| >= 20; after 12 terms the
// first omitted term is below 1e-20 of the leading one.
fn log_lower_tail(x: f64) -> f64 {
    let inverse_square = 1.0 / (x * x);
    let mut term = 1.0;
    let mut sum = 1.0;
    for n in 1..=12 {
        term *= -f64::from(2 * n - 1) * inverse_square;
        sum += term;
    }
    -0.5 * x * x - libm::log(-x) - 0.5 * libm::log(core::f64::consts::TAU) + libm::log(sum)
}

pub(super) fn invalid(field: &str, reason: &str) -> ReadingsError {
    ReadingsError::InvalidValue {
        field: field.to_owned(),
        reason: reason.to_owned(),
    }
}
