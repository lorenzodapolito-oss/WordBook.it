//! The "where am I" hint while scrolling: a small label beside the scroll bar that names the
//! page and the heading the view has reached, like the tooltip Word shows while the scroll bar
//! thumb is dragged. It appears when the person scrolls (wheel, touchpad, middle-click scrolling
//! or dragging the bar), not when the view follows the caret while typing, and fades after the
//! scrolling stops.

use std::sync::Arc;

use egui::{Align2, Id, Order, Pos2, Rect, Stroke, pos2};
use wordcraft_doc::{Document, Path, StoryRef};
use wordcraft_layout::{DocLayout, Placed};

use crate::WordApp;
use crate::theme::{Tokens, regular};

/// Seconds the hint stays after the last scroll movement.
pub const LINGER: f64 = 1.2;
/// Screen points below the top edge of the view that count as "where the view is".
const PROBE_BELOW_TOP: f32 = 24.0;
/// Longest heading shown, in characters; the rest is cut with an ellipsis.
const MAX_HEADING_CHARS: usize = 70;

/// A heading of the body: paragraph path, outline level (0 = Heading 1), text.
#[derive(Clone, Debug, PartialEq)]
pub struct Heading {
    pub path: Path,
    pub level: u8,
    pub text: String,
}

/// The headings of the body in document order (the Navigation pane's list).
pub fn body_headings(doc: &Document) -> Vec<Heading> {
    let mut out = Vec::new();
    for path in doc.para_paths(StoryRef::Body) {
        let Some(p) = doc.para(StoryRef::Body, &path) else { continue };
        let rp = doc.styles.resolve_para(&p.props);
        let Some(level) = rp.outline_level else { continue };
        let text = p.plain_text().trim().to_string();
        if text.is_empty() || rp.style.starts_with("TOC") {
            continue;
        }
        out.push(Heading { path, level, text });
    }
    out
}

/// Where the view is: the page (index and displayed number) and the heading it is under.
#[derive(Clone, Debug, PartialEq)]
pub struct Located {
    pub page: usize,
    pub number: u32,
    pub pages: usize,
    pub heading: Option<String>,
}

/// Top of the first placed piece of a heading's paragraph: (page, y in page points).
fn heading_top(layout: &DocLayout, path: &Path) -> Option<(usize, f32)> {
    let &(page, item) = layout.index.get(&(StoryRef::Body, path.clone()))?.first()?;
    match layout.pages.get(page)?.items.get(item)? {
        Placed::Lines { y, .. } => Some((page, *y)),
        _ => None,
    }
}

/// The last heading that starts at or before `(page, y)`.
pub fn heading_at<'a>(layout: &DocLayout, headings: &'a [Heading], page: usize, y: f32) -> Option<&'a Heading> {
    let mut found = None;
    for h in headings {
        let Some((hp, hy)) = heading_top(layout, &h.path) else { continue };
        if hp < page || (hp == page && hy <= y) {
            found = Some(h);
        } else if hp > page {
            break;
        }
    }
    found
}

/// The page and heading at a point in page coordinates.
pub fn locate(layout: &DocLayout, headings: &[Heading], page: usize, y: f32) -> Option<Located> {
    let page = page.min(layout.pages.len().checked_sub(1)?);
    let number = layout.pages.get(page)?.number;
    Some(Located { page, number, pages: layout.pages.len(), heading: heading_at(layout, headings, page, y).map(|h| h.text.clone()) })
}

/// The text of the hint: "Page 3 of 12" and, when there is one, " · Heading".
pub fn label(at: &Located) -> String {
    let page = crate::i18n::fmt(tl!("Page {page} of {pages}"), &[("page", &at.number.to_string()), ("pages", &at.pages.to_string())]);
    match &at.heading {
        Some(h) => format!("{page}  ·  {}", clip(h)),
        None => page,
    }
}

fn clip(text: &str) -> String {
    let one_line: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= MAX_HEADING_CHARS {
        return one_line;
    }
    let cut: String = one_line.chars().take(MAX_HEADING_CHARS - 1).collect();
    format!("{}…", cut.trim_end())
}

/// What the canvas keeps between frames.
#[derive(Default)]
pub struct ScrollHint {
    last_y: f32,
    /// Input time until which the hint shows.
    shown_until: f64,
    /// The headings, rebuilt when the document changes.
    headings: Option<(u64, Arc<Vec<Heading>>)>,
}

impl ScrollHint {
    /// Note this frame's scroll offset; `by_person` is whether the person scrolled (not the view
    /// following the caret). Returns whether the hint shows.
    pub fn note(&mut self, now: f64, offset_y: f32, by_person: bool) -> bool {
        if (offset_y - self.last_y).abs() > 0.5 && by_person {
            self.shown_until = now + LINGER;
        }
        self.last_y = offset_y;
        now < self.shown_until
    }

    fn headings(&mut self, app: &WordApp) -> Arc<Vec<Heading>> {
        let rev = app.session.rev();
        match &self.headings {
            Some((r, h)) if *r == rev => h.clone(),
            _ => {
                let h = Arc::new(body_headings(&app.session.doc));
                self.headings = Some((rev, h.clone()));
                h
            }
        }
    }
}

/// Where the hint sits: right of the view's middle column, level with the scroll bar thumb.
fn anchor(view: Rect, offset_y: f32, max_y: f32) -> Pos2 {
    let t = if max_y > 0.0 { (offset_y / max_y).clamp(0.0, 1.0) } else { 0.0 };
    pos2(view.max.x - 24.0, view.min.y + 20.0 + t * (view.height() - 40.0).max(0.0))
}

/// Called by the canvas once per frame, after the scroll area. `by_person`: the wheel, the
/// touchpad or middle-click scrolling moved the view this frame.
pub fn show(app: &mut WordApp, ui: &mut egui::Ui, view: Rect, by_person: bool) {
    let now = ui.input(|i| i.time);
    let offset_y = app.canvas.scroll_offset.y;
    // Dragging the scroll bar: the primary button is down over its strip and the view moved.
    let on_bar = ui.input(|i| i.pointer.primary_down() && i.pointer.latest_pos().is_some_and(|p| view.contains(p) && p.x >= view.max.x - 18.0));
    let mut hint = std::mem::take(&mut app.canvas.scroll_hint);
    let visible = hint.note(now, offset_y, by_person || on_bar);
    let headings = if visible { Some(hint.headings(app)) } else { None };
    app.canvas.scroll_hint = hint;
    let Some(headings) = headings else { return };
    if app.canvas.page_rects.is_empty() {
        return;
    }
    let layout = app.session.layout();
    let probe = pos2(view.center().x, view.min.y + PROBE_BELOW_TOP);
    let Some((page, _, y)) = crate::canvas::page_at(&app.canvas.page_rects, &layout, app.canvas.scale, probe) else { return };
    let Some(at) = locate(&layout, &headings, page, y) else { return };
    let text = label(&at);
    let t = Tokens::get(ui.ctx());
    let ctx = ui.ctx().clone();
    egui::Area::new(Id::new("scroll_hint"))
        .order(Order::Tooltip)
        .fixed_pos(anchor(view, offset_y, app.canvas.scroll_max.y))
        .pivot(Align2::RIGHT_CENTER)
        .interactable(false)
        .show(&ctx, |ui| {
            egui::Frame::NONE.fill(t.panel).stroke(Stroke::new(1.0, t.border)).corner_radius(6).inner_margin(egui::Margin::symmetric(10, 6)).show(
                ui,
                |ui| {
                    ui.add(egui::Label::new(egui::RichText::new(&text).font(regular(13.0)).color(t.text)).wrap_mode(egui::TextWrapMode::Extend));
                },
            );
        });
    // Come back when the linger is over, so the hint goes away without further input.
    ctx.request_repaint_after(std::time::Duration::from_millis(100));
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::vec2;

    fn heading(text: &str) -> Heading {
        Heading { path: Path::default(), level: 0, text: text.into() }
    }

    #[test]
    fn hint_shows_after_a_person_scrolls_and_fades() {
        let mut h = ScrollHint::default();
        assert!(!h.note(0.0, 0.0, false), "nothing at start");
        assert!(h.note(1.0, 100.0, true), "a wheel turn shows it");
        assert!(h.note(1.0 + LINGER - 0.1, 100.0, false), "it lingers");
        assert!(!h.note(1.0 + LINGER + 0.1, 100.0, false), "and fades");
        assert!(!h.note(10.0, 400.0, false), "the view following the caret does not show it");
        assert!(h.note(11.0, 500.0, true), "scrolling again does");
    }

    #[test]
    fn long_headings_are_cut_and_whitespace_collapsed() {
        assert_eq!(clip("  Due   righe\ndi titolo "), "Due righe di titolo");
        let long = "x".repeat(200);
        let c = clip(&long);
        assert_eq!(c.chars().count(), MAX_HEADING_CHARS);
        assert!(c.ends_with('…'));
    }

    #[test]
    fn label_names_page_and_heading() {
        let at = Located { page: 2, number: 3, pages: 12, heading: Some("Introduzione".into()) };
        assert_eq!(label(&at), "Page 3 of 12  ·  Introduzione");
        let none = Located { heading: None, ..at };
        assert_eq!(label(&none), "Page 3 of 12");
        let _ = heading("x");
    }

    #[test]
    fn anchor_follows_the_thumb_inside_the_view() {
        let view = Rect::from_min_size(pos2(0.0, 100.0), vec2(800.0, 600.0));
        assert_eq!(anchor(view, 0.0, 1000.0).y, 120.0);
        assert_eq!(anchor(view, 1000.0, 1000.0).y, 680.0);
        assert_eq!(anchor(view, 500.0, 0.0).y, 120.0, "nothing to scroll");
        assert!(anchor(view, 5000.0, 1000.0).y <= 680.0, "clamped");
    }

    #[test]
    fn sample_document_headings_are_found_and_located() {
        let mut s = wordcraft_engine::Session::new(wordcraft_engine::sample::sample_document());
        let headings = body_headings(&s.doc);
        assert!(headings.len() >= 2, "the sample has headings");
        let layout = s.layout();
        let at_top = locate(&layout, &headings, 0, 0.0).unwrap();
        assert_eq!((at_top.page, at_top.number), (0, 1));
        let last = layout.pages.len() - 1;
        let at_end = locate(&layout, &headings, last, f32::MAX).unwrap();
        assert_eq!(at_end.heading.as_deref(), headings.last().map(|h| h.text.as_str()));
        assert!(locate(&layout, &headings, 9999, 0.0).is_some(), "page index is clamped");
    }
}
