//! Route images for sharing or laying over photos, replacing the `{file_id}_track.png` files the
//! Python importer wrote (`processing/visualization.py::plot_track`): a white line on a
//! transparent background.

use tiny_skia::{LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

use super::outline::RouteOutline;
use crate::error::{AppError, AppResult};

/// Pixels along the longer side, like Python's 10 in × 300 dpi figure.
pub const PNG_SIZE: f32 = 3000.0;
/// Empty border, as a share of the longer side.
const MARGIN: f32 = 0.04;
/// Line width, as a share of the longer side.
const LINE: f32 = 0.004;

/// Canvas size (width, height) in pixels for a longer side of `size`.
fn canvas(outline: &RouteOutline, size: f32) -> (f32, f32) {
    let scale = size * (1.0 - 2.0 * MARGIN);
    let pad = size * MARGIN;
    (
        (outline.width as f32 * scale + 2.0 * pad).max(1.0),
        (outline.height as f32 * scale + 2.0 * pad).max(1.0),
    )
}

fn project(outline: &RouteOutline, size: f32) -> impl Iterator<Item = (f32, f32)> + '_ {
    let scale = size * (1.0 - 2.0 * MARGIN);
    let pad = size * MARGIN;
    outline
        .points
        .iter()
        .map(move |[x, y]| (*x as f32 * scale + pad, *y as f32 * scale + pad))
}

/// PNG bytes: white route, transparent background.
pub fn png(outline: &RouteOutline) -> AppResult<Vec<u8>> {
    let (width, height) = canvas(outline, PNG_SIZE);
    let mut pixmap = Pixmap::new(width.round() as u32, height.round() as u32)
        .ok_or_else(|| AppError::Invalid("route image has no size".into()))?;
    let mut path = PathBuilder::new();
    for (i, (x, y)) in project(outline, PNG_SIZE).enumerate() {
        if i == 0 {
            path.move_to(x, y);
        } else {
            path.line_to(x, y);
        }
    }
    let path = path
        .finish()
        .ok_or_else(|| AppError::Invalid("route has too few points".into()))?;
    let mut paint = Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    paint.anti_alias = true;
    let stroke = Stroke {
        width: PNG_SIZE * LINE,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    pixmap
        .encode_png()
        .map_err(|e| AppError::Invalid(format!("could not encode PNG: {e}")))
}

/// Standalone SVG with the same look, at 1000 units along the longer side.
pub fn svg(outline: &RouteOutline) -> String {
    const SIZE: f32 = 1000.0;
    let (width, height) = canvas(outline, SIZE);
    let points = project(outline, SIZE)
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0}" height="{height:.0}" viewBox="0 0 {width:.1} {height:.1}"><polyline points="{points}" fill="none" stroke="#ffffff" stroke-width="{line:.1}" stroke-linecap="round" stroke-linejoin="round"/></svg>
"##,
        line = SIZE * LINE
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> RouteOutline {
        RouteOutline {
            width: 1.0,
            height: 0.5,
            points: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 0.5], [0.0, 0.5]],
        }
    }

    #[test]
    fn png_keeps_the_aspect_ratio_and_is_transparent() {
        let bytes = png(&square()).unwrap();
        let image = Pixmap::decode_png(&bytes).unwrap();
        assert_eq!(image.width(), 3000);
        assert_eq!(image.height(), 1620);
        // Corner is transparent, a point on the line is white.
        assert_eq!(image.pixel(0, 0).unwrap().alpha(), 0);
        let on_line = image.pixel(1500, 120).unwrap();
        assert_eq!((on_line.red(), on_line.alpha()), (255, 255));
    }

    #[test]
    fn svg_draws_one_polyline() {
        let svg = svg(&square());
        assert!(svg.starts_with("<svg"));
        assert_eq!(svg.matches("<polyline").count(), 1);
        assert!(svg.contains(r#"width="1000" height="540""#));
    }
}
