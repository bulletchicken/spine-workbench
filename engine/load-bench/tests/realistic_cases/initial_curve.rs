use load_bench::rod_beams::polyline_stations;
use nalgebra::Vector3;

/// This project's bounded work for each short initial-projection interval.
const INITIAL_STATION_STEPS: usize = 64;

/// Initialisation only: direct Hermite evaluation on the supplied samples.
/// Each dense <=2 mm interval is searched separately with 64 golden steps,
/// also testing every endpoint. No solver constraint/Jacobian is reused.
pub(super) struct InitialCurve<'a> {
    points: &'a [Vector3<f64>],
    knots: Vec<f64>,
    slopes: Vec<Vector3<f64>>,
}

impl<'a> InitialCurve<'a> {
    pub(super) fn new(points: &'a [Vector3<f64>]) -> Result<Self, String> {
        let knots = polyline_stations(points).map_err(|e| format!("initial contour: {e}"))?;
        let slopes = (0..points.len())
            .map(|i| {
                let a = i.saturating_sub(1);
                let b = (i + 1).min(points.len() - 1);
                (points[b] - points[a]) / (knots[b] - knots[a])
            })
            .collect();
        Ok(Self {
            points,
            knots,
            slopes,
        })
    }
    pub(super) fn length(&self) -> f64 {
        *self.knots.last().unwrap()
    }
    fn point(&self, i: usize, t: f64) -> Vector3<f64> {
        let h = self.knots[i + 1] - self.knots[i];
        self.points[i] * ((1.0 + 2.0 * t) * (1.0 - t) * (1.0 - t))
            + self.slopes[i] * (h * t * (1.0 - t) * (1.0 - t))
            + self.points[i + 1] * (t * t * (3.0 - 2.0 * t))
            + self.slopes[i + 1] * (h * t * t * (t - 1.0))
    }
    pub(super) fn closest_station(&self, head: Vector3<f64>) -> f64 {
        let mut best = (f64::INFINITY, 0.0);
        for i in 0..self.points.len() - 1 {
            let distance = |t: f64| (self.point(i, t) - head).norm_squared();
            let (mut a, mut b) = (0.0, 1.0);
            let golden = (5.0_f64.sqrt() - 1.0) * 0.5;
            let (mut c, mut d) = (b - golden * (b - a), a + golden * (b - a));
            let (mut fc, mut fd) = (distance(c), distance(d));
            for _ in 0..INITIAL_STATION_STEPS {
                if fc < fd {
                    b = d;
                    d = c;
                    fd = fc;
                    c = b - golden * (b - a);
                    fc = distance(c);
                } else {
                    a = c;
                    c = d;
                    fc = fd;
                    d = a + golden * (b - a);
                    fd = distance(d);
                }
            }
            for t in [0.0, 1.0, (a + b) * 0.5] {
                let candidate = distance(t);
                if candidate < best.0 {
                    best = (
                        candidate,
                        self.knots[i] + t * (self.knots[i + 1] - self.knots[i]),
                    );
                }
            }
        }
        best.1
    }
}
