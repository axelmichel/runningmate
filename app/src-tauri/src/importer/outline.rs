//! Simplified route shape for the activity panel, replacing the Python `_track.png` image.

use serde::Serialize;
use ts_rs::TS;

use super::{tcx::RawPoint, track};

/// Route points projected to metres and scaled into a box of `width × height` (the longer side
/// is 1), y pointing down for SVG.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RouteOutline {
    pub width: f64,
    pub height: f64,
    pub points: Vec<[f64; 2]>,
}

/// Ramer–Douglas–Peucker simplification of a polyline.
pub(crate) fn simplify(points: &[(f64, f64)], epsilon: f64) -> Vec<(f64, f64)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let (first, last) = (points[0], points[points.len() - 1]);
    let (dx, dy) = (last.0 - first.0, last.1 - first.1);
    let length = (dx * dx + dy * dy).sqrt();
    let distance = |p: &(f64, f64)| {
        if length == 0.0 {
            ((p.0 - first.0).powi(2) + (p.1 - first.1).powi(2)).sqrt()
        } else {
            (dy * p.0 - dx * p.1 + last.0 * first.1 - last.1 * first.0).abs() / length
        }
    };
    let (index, max) = points[1..points.len() - 1]
        .iter()
        .enumerate()
        .map(|(i, p)| (i + 1, distance(p)))
        .fold(
            (0, 0.0),
            |best, item| if item.1 > best.1 { item } else { best },
        );
    if max > epsilon {
        let mut left = simplify(&points[..=index], epsilon);
        let right = simplify(&points[index..], epsilon);
        left.pop();
        left.extend(right);
        left
    } else {
        vec![first, last]
    }
}

pub fn outline(points: &[RawPoint]) -> Option<RouteOutline> {
    if points.len() < 2 {
        return None;
    }
    let xy = track::utm_coordinates(points);
    let (min_x, max_x) = xy.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p.0), hi.max(p.0))
    });
    let (min_y, max_y) = xy.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p.1), hi.max(p.1))
    });
    let extent = (max_x - min_x).max(max_y - min_y);
    if extent <= 0.0 {
        return None;
    }
    // 0.2 % of the route's size keeps the shape and leaves a few hundred points.
    let simplified = simplify(&xy, extent * 0.002);
    Some(RouteOutline {
        width: (max_x - min_x) / extent,
        height: (max_y - min_y) / extent,
        points: simplified
            .iter()
            .map(|(x, y)| [(x - min_x) / extent, (max_y - y) / extent])
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_collinear_points() {
        let line = [(0.0, 0.0), (1.0, 0.0), (2.0, 0.001), (3.0, 0.0), (3.0, 3.0)];
        assert_eq!(
            simplify(&line, 0.01),
            vec![(0.0, 0.0), (3.0, 0.0), (3.0, 3.0)]
        );
    }
}
