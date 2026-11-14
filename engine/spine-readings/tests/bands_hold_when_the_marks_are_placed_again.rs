//! Repeated independent marked films test the bands against exact-mark angles.

mod noisy_films;

use joint_fit::fit_from_films;
use noisy_films::{RECIPES, exact, input, main_curve, noisy, overlap, values};
use shapes::made_up::RecipeName;
use spine_readings::measure_spine;

fn measure(seeds: std::ops::Range<u64>) {
    for name in RECIPES {
        let request = exact(name, false);
        let mut exact_input = input(&request, fit_from_films(&request).unwrap());
        // Bands are closed form; this test does not use the version counts.
        exact_input.versions = 1;
        let truth = measure_spine(&exact_input).unwrap();
        let main = &truth.curves[0];
        let expected = values(main.cobb).unwrap().0;
        let mut count = 0;
        let mut inside = 0;
        let mut found = 0;
        let mut total_error = 0.0;
        let mut total_half_width = 0.0;
        let mut unmatched = 0;
        for seed in seeds.clone() {
            let marked = noisy(&request, seed, 1.0, true);
            let fit = fit_from_films(&marked).unwrap();
            assert!(fit.converged, "{name:?} seed {seed}");
            let mut noisy_input = input(&marked, fit);
            noisy_input.versions = 1;
            let measured = measure_spine(&noisy_input).unwrap();
            unmatched += measured
                .curves
                .iter()
                .filter(|curve| !truth.curves.iter().any(|exact| overlap(curve, exact)))
                .count();
            if let Some(curve) = main_curve(&measured, main) {
                let (value, half_width) = values(curve.cobb).unwrap();
                let error = value.abs() - expected.abs();
                count += 1;
                inside += u32::from(error.abs() <= half_width);
                found += u32::from(overlap(&curve, main));
                total_error += error;
                total_half_width += half_width;
            }
        }
        let coverage = f64::from(inside) / f64::from(count);
        let mean_error = total_error / f64::from(count);
        eprintln!(
            "{name:?}: coverage={inside}/{count}, mean_error={mean_error:.6}, mean_half_width={:.6}, main_found={found}/{}, unmatched_per_spine_against_current_exact={:.6}",
            total_half_width / f64::from(count),
            (seeds.end - seeds.start),
            unmatched as f64 / (seeds.end - seeds.start) as f64
        );
        assert!(
            (0.72..=0.90).contains(&coverage),
            "{name:?}: {inside}/{count}"
        );
        // Choosing the most tilted plates on noisy marks also chooses the
        // noise being measured: the known main-thoracic effect is upward bias.
        if name != RecipeName::LumbarLeft30 {
            assert!(mean_error > 0.0, "{name:?}: mean error {mean_error}");
        }
    }
}

#[test]
#[ignore = "2700 repeated film fits; run in release mode"]
fn nine_hundred_fresh_markings_per_spine_keep_the_main_angle_band() {
    measure(120_000..120_900);
}
