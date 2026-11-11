//! Carry unpinned pedicles through a changed body's bilinear coordinates.

use shapes::marks::{MarkSource, PedicleMark};
use shapes::pixels::Px;

fn cross(a: Px, b: Px) -> f64 {
    a.x() * b.y() - a.y() * b.x()
}

fn difference(a: Px, b: Px) -> Px {
    Px::new(a.x() - b.x(), a.y() - b.y())
}

fn sample(q: [Px; 4], u: f64, v: f64) -> Px {
    let weights = [(1.0 - u) * (1.0 - v), u * (1.0 - v), u * v, (1.0 - u) * v];
    Px::new(
        (0..4).map(|i| weights[i] * q[i].x()).sum(),
        (0..4).map(|i| weights[i] * q[i].y()).sum(),
    )
}

fn inverse(q: [Px; 4], point: Px) -> Option<(f64, f64)> {
    let b = difference(q[1], q[0]);
    let c = difference(q[3], q[0]);
    let d = Px::new(
        q[0].x() - q[1].x() + q[2].x() - q[3].x(),
        q[0].y() - q[1].y() + q[2].y() - q[3].y(),
    );
    let e = difference(point, q[0]);
    let a = -cross(b, d);
    let middle = cross(e, d) - cross(b, c);
    let constant = cross(e, c);
    let roots = if a.abs() < 1e-12 {
        if middle.abs() < 1e-12 {
            return None;
        }
        [-constant / middle, f64::NAN]
    } else {
        let discriminant = middle * middle - 4.0 * a * constant;
        if discriminant < 0.0 {
            return None;
        }
        let root = discriminant.sqrt();
        let numerator = -0.5 * (middle + root.copysign(middle));
        if numerator == 0.0 {
            [-middle / (2.0 * a), f64::NAN]
        } else {
            [numerator / a, constant / numerator]
        }
    };
    for u in roots {
        let denominator = Px::new(c.x() + u * d.x(), c.y() + u * d.y());
        let residual = Px::new(e.x() - u * b.x(), e.y() - u * b.y());
        let norm = denominator.x() * denominator.x() + denominator.y() * denominator.y();
        let v = (residual.x() * denominator.x() + residual.y() * denominator.y()) / norm;
        if (-0.5..=1.5).contains(&u) && (-0.5..=1.5).contains(&v) {
            return Some((u, v));
        }
    }
    None
}

pub(super) fn transfer(pedicle: &mut PedicleMark, old: [Px; 4], new: [Px; 4]) {
    if old == new {
        return;
    }
    let PedicleMark::Visible {
        at,
        pinned: false,
        source,
        inferred,
        ring_score,
    } = pedicle
    else {
        return;
    };
    if let Some((u, v)) = inverse(old, *at) {
        let next = sample(new, u, v);
        if next == *at {
            return;
        }
        *at = next;
    }
    *source = MarkSource::Auto;
    *inferred = true;
    *ring_score = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pedicle_keeps_its_bilinear_position_on_a_trapezoid() {
        // At (u,v)=(1/4,1/2), a 40 by 20 rectangle gives (10,10).
        // The new top edge is (0,0)..(60,0), bottom (10,40)..(50,40),
        // so the corresponding point is (17.5,20), independently by interpolation.
        let old = [
            Px::new(0.0, 0.0),
            Px::new(40.0, 0.0),
            Px::new(40.0, 20.0),
            Px::new(0.0, 20.0),
        ];
        let new = [
            Px::new(0.0, 0.0),
            Px::new(60.0, 0.0),
            Px::new(50.0, 40.0),
            Px::new(10.0, 40.0),
        ];
        let mut pedicle = PedicleMark::Visible {
            at: Px::new(10.0, 10.0),
            source: MarkSource::Auto,
            pinned: false,
            inferred: false,
            ring_score: Some(0.8),
        };
        transfer(&mut pedicle, old, new);
        assert_eq!(pedicle.mark().unwrap().at, Px::new(17.5, 20.0));
        assert!(pedicle.mark().unwrap().inferred);
        // Inverting the trapezoid exercises the quadratic, returning the
        // rectangle point to within 1e-10 px of floating-point roundoff.
        transfer(&mut pedicle, new, old);
        let at = pedicle.mark().unwrap().at;
        assert!((at.x() - 10.0).abs() < 1e-10 && (at.y() - 10.0).abs() < 1e-10);
    }
}
