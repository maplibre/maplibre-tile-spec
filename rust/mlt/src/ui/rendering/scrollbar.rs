use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};
use usize_cast::IntoUsize as _;

/// How much of a panel's content is visible, in the panel's own scroll units.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Scroll {
    pub content: usize,
    pub view: usize,
    pub pos: usize,
}

impl Scroll {
    pub(crate) fn new(content: usize, view: usize, pos: u16) -> Self {
        Self {
            content,
            view,
            pos: pos.into_usize(),
        }
    }

    /// Largest scroll position that still shows a full view of content.
    pub(crate) fn max_pos(content: usize, view: usize) -> u16 {
        u16::try_from(content.saturating_sub(view)).unwrap_or(u16::MAX)
    }
}

/// Draw a scrollbar on the right border of a bordered panel when its content does not fit.
pub(crate) fn render_vscrollbar(f: &mut Frame<'_>, area: Rect, scroll: Scroll) {
    let bar = area.inner(Margin {
        vertical: 1,
        horizontal: 0,
    });
    render_scrollbar(f, bar, ScrollbarOrientation::VerticalRight, scroll);
}

/// Draw a scrollbar on the bottom border of a bordered panel when its content does not fit.
pub(crate) fn render_hscrollbar(f: &mut Frame<'_>, area: Rect, scroll: Scroll) {
    let bar = area.inner(Margin {
        vertical: 0,
        horizontal: 1,
    });
    render_scrollbar(f, bar, ScrollbarOrientation::HorizontalBottom, scroll);
}

/// The thumb spans the visible share of the content.
fn render_scrollbar(
    f: &mut Frame<'_>,
    bar: Rect,
    orientation: ScrollbarOrientation,
    scroll: Scroll,
) {
    if scroll.content <= scroll.view {
        return;
    }
    let mut state = ScrollbarState::new(scroll.content)
        .position(scroll.pos)
        .viewport_content_length(scroll.view);
    f.render_stateful_widget(Scrollbar::new(orientation), bar, &mut state);
}

/// Rows the lines take when wrapped into `width` columns.
/// Word wrapping can add a row or two, so this is a lower bound.
pub(crate) fn wrapped_rows(lines: &[Line<'_>], width: usize) -> usize {
    let width = width.max(1);
    lines.iter().map(|l| l.width().max(1).div_ceil(width)).sum()
}
