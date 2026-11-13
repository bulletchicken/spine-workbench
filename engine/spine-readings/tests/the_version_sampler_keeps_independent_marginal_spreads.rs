//! Independent distribution, boundary and identity probes for shared versions.

use shapes::levels::Level;
use shapes::made_up::VertebraPose;
use shapes::spine_fit::{
    FitAngles, FitRmsMm, FitStandardDeviations, FitStopReason, FittedVertebra, InferredFit,
    SpineFit,
};
use spine_readings::readings::{NamedSampler, inverse_normal, version, versions};

fn nominal() -> SpineFit {
    SpineFit {
        vertebrae: [Level::T7, Level::T8, Level::T9]
            .into_iter()
            .enumerate()
            .map(|(i, level)| FittedVertebra {
                level,
                pose: VertebraPose {
                    level,
                    centre: [10.0, -20.0, 240.0 - 30.0 * i as f64],
                    rotation: [0.0; 3],
                    height: 20.0,
                    wedge_front: 0.0,
                    wedge_side: 0.0,
                    top_half_width: 15.0,
                    bottom_half_width: 15.0,
                    top_half_depth: 10.0,
                    bottom_half_depth: 10.0,
                },
                angles: FitAngles {
                    side_tilt_rad: 0.0,
                    forward_lean_rad: 0.0,
                    twist_rad: 0.0,
                },
                sd: FitStandardDeviations {
                    centre_mm: [0.5 + i as f64, 1.5 + i as f64, 2.5 + i as f64],
                    angles_rad: [0.005, 0.015, 0.025],
                },
                inferred: InferredFit {
                    depth: false,
                    width: false,
                    twist: false,
                },
                rms_mm: FitRmsMm {
                    front: Some(0.0),
                    side: Some(0.0),
                },
                corner_tilt_rad: None,
            })
            .collect(),
        side_scale: 1.0,
        side_offset_mm: 0.0,
        origin_level: Level::T9,
        iterations: 0,
        converged: true,
        stop_reason: FitStopReason::SmallStep,
        rms_mm: 0.0,
        lengths: shapes::spine_fit::FitLengths::Measured,
        typical_curve_deg: None,
        front_film: shapes::spine_fit::FilmPlacement {
            mm_per_px: [1.0; 2],
            origin_px: [0.0; 2],
            facing_assumed: false,
        },
        side_film: Some(shapes::spine_fit::FilmPlacement {
            mm_per_px: [1.0; 2],
            origin_px: [0.0; 2],
            facing_assumed: false,
        }),
    }
}

#[test]
fn quantiles_resolve_both_tails_to_a_nanounit() {
    // Independent Wichura AS241 quantiles, evaluated by Python's NormalDist.
    // The central values also agree with standard normal printed tables.
    let cases = [
        (0.001, -3.090_232_306_167_813),
        (0.025, -1.959_963_984_540_053_8),
        (0.5, 0.0),
        (0.975, 1.959_963_984_540_053_4),
        (0.999, 3.090_232_306_167_813),
        (1e-20, -9.262_340_089_798_407),
        (1e-100, -21.273_453_560_965_322),
        (1e-300, -37.047_096_299_361_2),
        (1e-310, -37.663_060_331_949_52),
        (1e-320, -38.269_125_343_032_655),
        (f64::from_bits(1), -38.467_405_617_144_344),
        (1.0_f64.next_down(), 8.209_536_151_601_386),
        (1.0 - 1.0 / 1_048_576.0, 4.763_001_034_267_813_5),
    ];
    for (p, expected) in cases {
        let actual = inverse_normal(p).unwrap();
        assert!((actual - expected).abs() < 1e-9, "p={p}: {actual}");
    }
    // Dyadic p and 1-p are represented exactly, so odd symmetry is exact.
    for exponent in 1..=52 {
        let p = 1.0 / ((1_u64 << exponent) as f64);
        assert_eq!(
            inverse_normal(p).unwrap(),
            -inverse_normal(1.0 - p).unwrap()
        );
    }
}

#[test]
fn single_versions_keep_their_bits_across_seeds_sizes_and_batch_positions() {
    let fit = nominal();
    let frozen = serde_json::to_vec(&fit).unwrap();
    for seed in [0, 1, 33, u64::MAX] {
        for n in [1, 2, 3, 7, 32] {
            let complete = versions(&fit, seed, n, 0..n).unwrap();
            assert_eq!(serde_json::to_vec(&complete.nominal).unwrap(), frozen);
            assert!(version(&fit, seed, n, n).is_err());
            for k in 0..n {
                let single = version(&fit, seed, n, k).unwrap();
                let split = versions(&fit, seed, n, k..k + 1).unwrap();
                assert_eq!(single, complete.versions[k as usize]);
                assert_eq!(single, split.versions[0]);
                assert_eq!(
                    serde_json::to_vec(&single.fit).unwrap(),
                    serde_json::to_vec(&complete.versions[k as usize].fit).unwrap()
                );
                assert_eq!(
                    single.z_child.to_bits(),
                    split.versions[0].z_child.to_bits()
                );
                for (left, right) in single.pedicles.iter().zip(&split.versions[0].pedicles) {
                    assert_eq!(left.z_pedicle.to_bits(), right.z_pedicle.to_bits());
                }
                for (left, right) in single
                    .fit
                    .vertebrae
                    .iter()
                    .zip(&split.versions[0].fit.vertebrae)
                {
                    for (a, b) in left
                        .pose
                        .centre
                        .iter()
                        .chain(&left.pose.rotation)
                        .zip(right.pose.centre.iter().chain(&right.pose.rotation))
                    {
                        assert_eq!(a.to_bits(), b.to_bits());
                    }
                }
            }
        }
    }
    assert_eq!(serde_json::to_vec(&fit).unwrap(), frozen);
    let one = NamedSampler::new(0, 1).unwrap();
    assert!(one.uniform("", 0).unwrap() > 0.0);
    assert!(one.uniform("", 0).unwrap() < 1.0);
    assert!(one.uniform("", 1).is_err());
    assert!(NamedSampler::new(0, 0).is_err());
}

#[test]
#[ignore = "many-seed distribution sweep"]
fn names_prefixes_and_seeds_have_independent_draws_and_exact_strata() {
    // Independent uniforms have variance 1/12 and zero covariance. Pooling
    // 32768 rows gives a correlation standard error near 0.006; 0.04 is
    // over six such errors, but excludes reused or strongly coupled streams.
    let mut left = Vec::new();
    let mut prefix = Vec::new();
    let mut other = Vec::new();
    let mut next_seed = Vec::new();
    for seed in 0..1_024 {
        let sampler = NamedSampler::new(seed, 32).unwrap();
        let changed = NamedSampler::new(seed + 1, 32).unwrap();
        let mut counts = [[0; 32]; 4];
        for k in 0..32 {
            for (column, name) in ["a", "ab", "unrelated", ""].into_iter().enumerate() {
                let u = sampler.uniform(name, k).unwrap();
                counts[column][libm::floor(32.0 * u) as usize] += 1;
                match column {
                    0 => left.push(u),
                    1 => prefix.push(u),
                    2 => other.push(u),
                    _ => {}
                }
            }
            next_seed.push(changed.uniform("a", k).unwrap());
        }
        assert!(counts.iter().flatten().all(|&count| count == 1));
    }
    for comparison in [&prefix, &other, &next_seed] {
        let correlation = correlation(&left, comparison);
        assert!(correlation.abs() < 0.04, "correlation={correlation}");
        assert_ne!(left, *comparison);
    }
}

fn correlation(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    let mean_a = a.iter().sum::<f64>() / n;
    let mean_b = b.iter().sum::<f64>() / n;
    let covariance = a
        .iter()
        .zip(b)
        .map(|(x, y)| (x - mean_a) * (y - mean_b))
        .sum::<f64>();
    let variance_a = a.iter().map(|x| (x - mean_a) * (x - mean_a)).sum::<f64>();
    let variance_b = b.iter().map(|y| (y - mean_b) * (y - mean_b)).sum::<f64>();
    covariance / libm::sqrt(variance_a * variance_b)
}

#[test]
#[ignore = "many-version marginal-spread sweep"]
fn every_vertebra_and_axis_has_its_stated_marginal_sd() {
    let fit = nominal();
    let original = serde_json::to_vec(&fit).unwrap();
    let mut columns = vec![Vec::new(); 18];
    // Independent standard normals have mean 0 and variance 1. These
    // bounds are a little over five normal-sample standard errors for
    // 8192 draws; a wrong axis, omitted noise, or factor of two fails.
    for seed in 0..256 {
        for sampled in versions(&fit, seed, 32, 0..32).unwrap().versions {
            for (i, (got, expected)) in sampled.fit.vertebrae.iter().zip(&fit.vertebrae).enumerate()
            {
                let angles = [
                    got.angles.side_tilt_rad,
                    got.angles.forward_lean_rad,
                    got.angles.twist_rad,
                ];
                for axis in 0..3 {
                    columns[6 * i + axis].push(
                        (got.pose.centre[axis] - expected.pose.centre[axis])
                            / expected.sd.centre_mm[axis],
                    );
                    columns[6 * i + 3 + axis].push(angles[axis] / expected.sd.angles_rad[axis]);
                }
                assert_eq!(got.sd, expected.sd);
                assert_eq!(got.inferred, expected.inferred);
                assert_eq!(got.pose.height, expected.pose.height);
            }
        }
    }
    for (column, values) in columns.iter().enumerate() {
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance =
            values.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / values.len() as f64;
        assert!(mean.abs() < 0.06, "column={column}, mean={mean}");
        assert!(
            (variance - 1.0).abs() < 0.09,
            "column={column}, variance={variance}"
        );
    }
    for i in 0..columns.len() {
        for j in i + 1..columns.len() {
            let correlation = correlation(&columns[i], &columns[j]);
            assert!(
                correlation.abs() < 0.07,
                "columns={i},{j}, correlation={correlation}"
            );
        }
    }
    assert_eq!(serde_json::to_vec(&fit).unwrap(), original);
}
