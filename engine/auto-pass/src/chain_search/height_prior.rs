//! Body-height and disc-height priors, independent of the observed film.
//!
//! Reference means are from Busscher et al., 2010, Table 3
//! (PMC2900026). Interpreting each IDH as the disc below
//! that body is an assumption. All weights, spreads, the peripheral-disc
//! factor, and the adolescent adjustments are experimental.

use shapes::auto_pass::Population;
use shapes::levels::Level;
use shapes::naming::Template;

/// A template's ratios and absolute sizes. Length arguments share any one
/// unit for ratios; `absolute` alone requires physical length units.
#[derive(Clone, Debug)]
pub struct HeightPrior {
    levels: Vec<Level>,
    population: Population,
    scale_assumed: bool,
}

impl HeightPrior {
    pub fn new(template: Template, population: Population, scale_assumed: bool) -> Self {
        Self {
            levels: template.levels(),
            population,
            scale_assumed,
        }
    }

    /// Cost of one body's size; disabled when the scale is assumed.
    pub fn absolute(&self, body: usize, height: f64) -> f64 {
        if self.scale_assumed {
            return 0.0;
        }
        let factor = if self.population == Population::Adolescent {
            0.85
        } else {
            1.0
        };
        0.2 * truncated_square(log_ratio(height, factor * mean_height(self.levels[body])) / 0.35)
    }

    /// Cost of the lower body's height relative to the upper body's.
    pub fn ratio(&self, transition: usize, height: f64, next_height: f64) -> f64 {
        let upper = self.levels[transition];
        let lower = self.levels[transition + 1];
        let expected = if lower == Level::L6 {
            -0.03
        } else {
            libm::log(mean_height(lower) / mean_height(upper))
        };
        let spread = if matches!(
            upper,
            Level::C7 | Level::T1 | Level::T2 | Level::L3 | Level::L4
        ) {
            0.18
        } else {
            0.12
        };
        truncated_square((log_ratio(next_height, height) - expected) / spread)
    }

    /// The disc term depends on the gap and the lower height only. This
    /// separability permits the exact split recurrence.
    pub fn disc(&self, transition: usize, gap: f64, next_height: f64) -> f64 {
        let upper = self.levels[transition];
        let lower = self.levels[transition + 1];
        let mut expected = match (upper, lower) {
            (Level::L5, Level::L6) => 0.8 * 12.7 / 27.2,
            (Level::T12, Level::T13) => 0.8 * 6.1 / 27.05,
            (Level::T13 | Level::T11, Level::L1) => 0.8 * 9.3 / 27.0,
            _ => 0.8 * disc_below(upper) / mean_height(lower),
        };
        if self.population == Population::Adolescent
            && matches!(
                upper,
                Level::C7
                    | Level::T1
                    | Level::T2
                    | Level::T3
                    | Level::T4
                    | Level::T5
                    | Level::T6
                    | Level::T7
                    | Level::T8
                    | Level::T9
                    | Level::T10
                    | Level::T11
                    | Level::T12
            )
            && lower != Level::L1
        {
            expected *= 1.2;
        }
        disc_cost(gap, next_height, expected)
    }

    /// The C6/C7 disc, relative to C7.
    pub fn start(&self, gap: f64, height: f64) -> f64 {
        disc_cost(gap, height, 0.8 * 4.4 / 15.8)
    }

    /// The mobile-spine/sacrum disc, relative to the last mobile body.
    pub fn end(&self, gap: f64, height: f64) -> f64 {
        disc_cost(gap, height, 0.259)
    }
}

fn disc_cost(gap: f64, height: f64, expected: f64) -> f64 {
    0.5 * truncated_square((log_ratio(gap, height) - libm::log(expected)) / 0.35)
}

fn log_ratio(numerator: f64, denominator: f64) -> f64 {
    if numerator > 0.0 && denominator > 0.0 {
        libm::log(numerator / denominator)
    } else {
        f64::INFINITY
    }
}

fn truncated_square(value: f64) -> f64 {
    (value * value).min(9.0)
}

/// Mean of the anterior and posterior reference body heights, in mm.
/// Variant T13 repeats T12; variant L6 continues the experimental −0.03
/// log ratio. No interpolation is applied to a missing level.
pub fn mean_height(level: Level) -> f64 {
    match level {
        Level::C1 | Level::C2 | Level::C3 | Level::C4 | Level::C5 | Level::C6 => 13.95,
        Level::C7 => 15.8,
        Level::T1 => 18.05,
        Level::T2 => 18.5,
        Level::T3 => 19.45,
        Level::T4 => 20.25,
        Level::T5 => 20.05,
        Level::T6 => 20.4,
        Level::T7 => 21.1,
        Level::T8 => 21.4,
        Level::T9 => 22.55,
        Level::T10 => 24.3,
        Level::T11 => 25.65,
        Level::T12 | Level::T13 => 27.05,
        Level::L1 => 27.0,
        Level::L2 => 28.55,
        Level::L3 => 29.25,
        Level::L4 => 27.9,
        Level::L5 => 27.2,
        Level::L6 | Level::L7 => 27.2 * libm::exp(-0.03),
    }
}

fn disc_below(level: Level) -> f64 {
    match level {
        Level::C1 | Level::C2 | Level::C3 | Level::C4 | Level::C5 | Level::C6 => 4.4,
        Level::C7 | Level::T3 | Level::T4 => 4.7,
        Level::T1 => 5.0,
        Level::T2 => 4.5,
        Level::T5 | Level::T9 => 4.8,
        Level::T6 => 5.3,
        Level::T7 => 5.2,
        Level::T8 => 4.9,
        Level::T10 => 5.9,
        Level::T11 => 6.1,
        Level::T12 | Level::T13 => 9.3,
        Level::L1 => 10.3,
        Level::L2 => 11.5,
        Level::L3 => 11.8,
        Level::L4 => 12.7,
        Level::L5 | Level::L6 | Level::L7 => 8.8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_ratios_have_zero_cost_and_are_scale_free() {
        let prior = HeightPrior::new(Template::Standard, Population::Adult, false);
        // C7→T1: (17.3+18.8)/2 divided by (15.7+15.9)/2.
        assert!(prior.ratio(0, 15.8, 18.05).abs() < 1e-25);
        assert!(prior.ratio(0, 31.6, 36.1).abs() < 1e-25);
        // C7/T1 peripheral disc: 0.8×4.7 mm. Tolerance is floating rounding only.
        assert!(prior.disc(0, 3.76, 18.05).abs() < 1e-25);
        assert!(prior.start(3.52, 15.8).abs() < 1e-25);
        assert!(prior.end(7.0448, 27.2).abs() < 1e-25);
    }

    #[test]
    fn assumed_scale_disables_absolute_sizes_only() {
        let prior = HeightPrior::new(Template::Standard, Population::Adolescent, true);
        assert_eq!(prior.absolute(0, 1000.0), 0.0);
        assert_eq!(prior.ratio(0, 1.0, 1000.0), 9.0);
        assert!(prior.disc(0, 3.76 * 1.2, 18.05).abs() < 1e-25);
    }

    #[test]
    fn variant_transitions_use_the_stated_disc_mapping() {
        let extra = HeightPrior::new(Template::ExtraLumbar, Population::Adult, false);
        assert!(extra.ratio(17, 27.2, 27.2 * libm::exp(-0.03)).abs() < 1e-25);
        assert!(extra.disc(17, 10.16, 27.2).abs() < 1e-25);
        let missing = HeightPrior::new(Template::MissingThoracic, Population::Adult, false);
        assert!(missing.disc(11, 7.44, 27.0).abs() < 1e-25);
        // The extra thoracic disc copies T11/T12, including the
        // adolescent adjustment; T13/L1 remains a lumbar-sized gap.
        let thoracic = HeightPrior::new(Template::ExtraThoracic, Population::Adolescent, false);
        assert!(thoracic.disc(12, 0.8 * 6.1 * 1.2, 27.05).abs() < 1e-25);
        assert!(thoracic.disc(13, 7.44, 27.0).abs() < 1e-25);
    }
}
