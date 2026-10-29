//! Alignment guides: while a picture, shape or text box is dragged, a thin line across the page
//! shows each place its left, centre or right edge (and top, middle or bottom) lines up with: the
//! page's edges and centre, the margins and the edges and centres of the other floating objects
//! on its page. The guides only *show* an alignment; they never move the object (no snapping).
//! WordCraft's own design (Word for Mac shows no guides).
//!
//! The maths here is pure (points in, points out) so it's tested without egui; `objects.rs`
//! gathers the candidates and draws the lines.

use wordcraft_geom::Rect;

/// How near (screen points) an edge or centre must be to a guide to count as lined up with it.
pub const ALIGN_SCREEN: f32 = 1.0;

/// Which way a guide runs: `X` is a vertical line at `at` (lefts, centres and rights line up with
/// it), `Y` a horizontal one (tops, middles and bottoms).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Axis {
    X,
    Y,
}

/// A place to line up with on the page, points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guide {
    pub axis: Axis,
    pub at: f32,
}

impl Guide {
    pub fn x(at: f32) -> Guide {
        Guide { axis: Axis::X, at }
    }
    pub fn y(at: f32) -> Guide {
        Guide { axis: Axis::Y, at }
    }
}

/// The guides `moving` (the dragged object's bounds, points) is lined up with: one of its left,
/// centre or right edges within `tolerance` points of an `X` guide, or its top, middle or bottom
/// of a `Y` guide. Each line once. Nothing moves.
pub fn aligned(moving: Rect, candidates: &[Guide], tolerance: f32) -> Vec<Guide> {
    let mut lines = Vec::new();
    if !tolerance.is_finite() || tolerance < 0.0 {
        return lines;
    }
    let xs = [moving.x, moving.x + moving.w / 2.0, moving.x + moving.w];
    let ys = [moving.y, moving.y + moving.h / 2.0, moving.y + moving.h];
    for g in candidates.iter().filter(|g| g.at.is_finite()) {
        let points = if g.axis == Axis::X { &xs } else { &ys };
        if points.iter().any(|p| p.is_finite() && (p - g.at).abs() <= tolerance) && !lines.contains(g) {
            lines.push(*g);
        }
    }
    lines
}

/// The guides of a page `w` × `h` points with text area `body`, and of the other floating objects
/// on it (their bounds): edges and centres.
pub fn candidates(w: f32, h: f32, body: Rect, others: impl IntoIterator<Item = Rect>) -> Vec<Guide> {
    let mut g = Vec::new();
    let mut add = |r: Rect| {
        for x in [r.x, r.x + r.w / 2.0, r.x + r.w] {
            g.push(Guide::x(x));
        }
        for y in [r.y, r.y + r.h / 2.0, r.y + r.h] {
            g.push(Guide::y(y));
        }
    };
    add(Rect::new(0.0, 0.0, w, h));
    add(body);
    for r in others {
        add(r);
    }
    g.retain(|g| g.at.is_finite());
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: Rect = Rect { x: 100.0, y: 200.0, w: 50.0, h: 20.0 };

    #[test]
    fn edges_and_centres_line_up() {
        assert_eq!(aligned(R, &[Guide::x(100.5)], 1.0), vec![Guide::x(100.5)]); // left
        assert_eq!(aligned(R, &[Guide::x(125.0)], 1.0), vec![Guide::x(125.0)]); // centre
        assert_eq!(aligned(R, &[Guide::x(150.0)], 1.0), vec![Guide::x(150.0)]); // right
        assert_eq!(aligned(R, &[Guide::y(200.0)], 1.0), vec![Guide::y(200.0)]); // top
        assert_eq!(aligned(R, &[Guide::y(210.0)], 1.0), vec![Guide::y(210.0)]); // middle
        assert_eq!(aligned(R, &[Guide::y(220.0)], 1.0), vec![Guide::y(220.0)]); // bottom
    }

    #[test]
    fn near_misses_show_nothing_and_nothing_moves() {
        assert!(aligned(R, &[Guide::x(97.0), Guide::y(230.0)], 1.0).is_empty());
        // an X guide is not matched against y values
        assert!(aligned(R, &[Guide::x(210.0)], 1.0).is_empty());
    }

    #[test]
    fn each_line_shows_once_and_junk_is_ignored() {
        let l = aligned(R, &[Guide::x(100.0), Guide::x(100.0), Guide::x(150.0)], 1.0);
        assert_eq!(l, vec![Guide::x(100.0), Guide::x(150.0)]);
        let junk = [Guide::x(f32::NAN), Guide::y(f32::INFINITY)];
        assert!(aligned(R, &junk, 1.0).is_empty());
        assert!(aligned(R, &[Guide::x(100.0)], f32::NAN).is_empty());
        assert!(aligned(R, &[Guide::x(100.0)], -1.0).is_empty());
        assert!(aligned(Rect { x: f32::NAN, y: 0.0, w: f32::NAN, h: 1.0 }, &[Guide::x(0.0)], 1.0).is_empty());
    }

    #[test]
    fn candidates_are_page_margins_and_other_objects() {
        let body = Rect::new(72.0, 72.0, 468.0, 648.0);
        let g = candidates(612.0, 792.0, body, [Rect::new(10.0, 20.0, 30.0, 40.0)]);
        for x in [0.0, 306.0, 612.0, 72.0, 540.0, 10.0, 25.0, 40.0] {
            assert!(g.contains(&Guide::x(x)), "x {x}: {g:?}");
        }
        for y in [0.0, 396.0, 792.0, 72.0, 720.0, 20.0, 40.0, 60.0] {
            assert!(g.contains(&Guide::y(y)), "y {y}: {g:?}");
        }
    }
}
