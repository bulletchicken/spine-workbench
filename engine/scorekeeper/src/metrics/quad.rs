//! Image-space quads, independent of the patient's left/right naming.

use serde::{Deserialize, Serialize};
use shapes::vertebra::VertebraMarks;

/// Four points in dataset storage order: top-left, top-right, bottom-left,
/// bottom-right. This differs from the outline order of shared marks.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Quad {
    pub tl: [f64; 2],
    pub tr: [f64; 2],
    pub bl: [f64; 2],
    pub br: [f64; 2],
}

impl Quad {
    pub fn from_marks(marks: &VertebraMarks) -> Self {
        let [tl, tr, br, bl] = marks.corners();
        Self {
            tl: tl.at.0,
            tr: tr.at.0,
            bl: bl.at.0,
            br: br.at.0,
        }
    }

    pub const fn points(&self) -> [[f64; 2]; 4] {
        [self.tl, self.tr, self.bl, self.br]
    }

    pub fn is_finite(&self) -> bool {
        self.points().iter().flatten().all(|v| v.is_finite())
    }

    /// Order each endplate by image x; never reorder top and bottom.
    pub fn x_canonical(&self) -> Self {
        let mut result = *self;
        if result.tl[0] > result.tr[0] {
            std::mem::swap(&mut result.tl, &mut result.tr);
        }
        if result.bl[0] > result.br[0] {
            std::mem::swap(&mut result.bl, &mut result.br);
        }
        result
    }

    pub fn height(&self) -> f64 {
        let q = self.x_canonical();
        (distance(q.tl, q.bl) + distance(q.tr, q.br)) * 0.5
    }

    pub fn centre(&self) -> [f64; 2] {
        [
            (self.tl[0] + self.tr[0] + self.bl[0] + self.br[0]) * 0.25,
            (self.tl[1] + self.tr[1] + self.bl[1] + self.br[1]) * 0.25,
        ]
    }

    /// Convex hull in counterclockwise numeric coordinates. Crossed and
    /// concave corner orderings therefore have the same well-defined area.
    pub fn hull(&self) -> Vec<[f64; 2]> {
        if !self.is_finite() {
            return Vec::new();
        }
        let mut points = self.points().to_vec();
        points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
        points.dedup();
        if points.len() <= 2 {
            return points;
        }
        let mut hull = Vec::new();
        for &p in &points {
            while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
                hull.pop();
            }
            hull.push(p);
        }
        let lower_len = hull.len();
        for &p in points.iter().rev().skip(1) {
            while hull.len() > lower_len
                && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0
            {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
        hull
    }
}

pub(crate) fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    libm::hypot(a[0] - b[0], a[1] - b[1])
}

pub(crate) fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
