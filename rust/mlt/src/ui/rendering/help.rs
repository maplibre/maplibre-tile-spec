use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};

use crate::ui::state::{App, ViewMode};
use crate::ui::{
    CLR_BAD_WINDING, CLR_DIMMED, CLR_EXTENT, CLR_HOVERED, CLR_INNER_RING, CLR_INNER_RING_SEL,
    CLR_LINE, CLR_MULTI_LINE, CLR_MULTI_POINT, CLR_MULTI_POLYGON, CLR_POINT, CLR_POLYGON,
    CLR_SELECTED, STYLE_LABEL, STYLE_SELECTED, block_with_title,
};

const CLR_ERROR: Color = Color::Red;

pub fn render_error_popup(f: &mut Frame<'_>, app: &App) {
    let Some((ref filename, ref msg)) = app.error_popup else {
        return;
    };
    let area = f.area();
    let lines: Vec<Line<'_>> = msg
        .trim()
        .lines()
        .map(|s| Line::from(s.to_string()))
        .collect();
    let line_count = lines.len();
    let height = u16::try_from(line_count)
        .unwrap_or(u16::MAX)
        .saturating_add(5)
        .min(28)
        .min(area.height.saturating_sub(4));
    let width = 80.min(area.width.saturating_sub(8));
    let popup = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    f.render_widget(ratatui::widgets::Clear, popup);
    let error_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(CLR_ERROR))
        .title_style(
            Style::default()
                .fg(CLR_ERROR)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )
        .title_top(format!(" Unable to open {filename} "))
        .title_bottom(Line::from("any key to close").right_aligned())
        .padding(Padding::uniform(1));
    let para = Paragraph::new(lines)
        .block(error_block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(para, popup);
}

pub fn render_help_overlay(f: &mut Frame<'_>, app: &mut App) {
    let area = f.area();
    let lines = match app.mode {
        ViewMode::FileBrowser => help_file_browser(),
        ViewMode::LayerOverview => help_layer_overview(),
    };
    let height = (u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .saturating_add(2))
    .min(area.height.saturating_sub(2));
    let width = 62.min(area.width.saturating_sub(4));
    let popup = Rect::new(
        area.x + (area.width.saturating_sub(width)) / 2,
        area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    let inner = height.saturating_sub(2);
    let max = u16::try_from(lines.len().saturating_sub(inner as usize)).unwrap_or(0);
    app.help_scroll = app.help_scroll.min(max);
    f.render_widget(ratatui::widgets::Clear, popup);
    let para = Paragraph::new(lines)
        .block(block_with_title(
            "Help (↑/↓/scroll to navigate, any other key to close)",
        ))
        .scroll((app.help_scroll, 0));
    f.render_widget(para, popup);
}

fn heading(text: &str) -> Line<'static> {
    Line::from(Span::styled(text.to_string(), STYLE_SELECTED))
}

fn key(k: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {k:<20}"), STYLE_LABEL),
        Span::raw(desc.to_string()),
    ])
}

fn color(c: Color, label: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{label:<20}"), Style::default().fg(c)),
        Span::raw(desc.to_string()),
    ])
}

fn help_file_browser() -> Vec<Line<'static>> {
    vec![
        heading("Keyboard"),
        key("?  h  F1", "Toggle this help"),
        key("q  Ctrl+c", "Quit"),
        key("Up/Down  j/k", "Navigate file list"),
        key("PageUp/PageDown", "Scroll by page"),
        key("Home/End", "Jump to first/last"),
        key("Enter", "Open selected MLT file"),
        key("Esc", "Quit"),
        Line::from(""),
        heading("Mouse"),
        key("Click row", "Select file"),
        key("Double-click row", "Open file"),
        key("Click header", "Sort by column"),
        key("Scroll", "Navigate file list"),
        key("Drag divider", "Resize panels"),
        Line::from(""),
        heading("Filter Panel"),
        key("Click checkbox", "Toggle geometry/algorithm filter"),
        key("Click [Reset]", "Clear all filters"),
    ]
}

fn help_layer_overview() -> Vec<Line<'static>> {
    vec![
        heading("Keyboard"),
        key("?  h  F1", "Toggle this help"),
        key("q  Ctrl+c", "Quit"),
        key("Esc", "Back to file browser"),
        key("Up/Down  j/k", "Navigate feature tree"),
        key("PageUp/PageDown", "Scroll by page"),
        key("Home/End", "Jump to first/last"),
        key("Enter", "Expand/collapse layer or feature"),
        key("+  =  Right", "Expand selected node"),
        key("-", "Collapse (or jump to parent)"),
        key("*", "Expand/collapse all layers"),
        key("Left", "Jump to parent node"),
        key("Ctrl+h / Ctrl+l", "Resize left/right split"),
        key("Shift+J / Shift+K", "Resize top/bottom split"),
        Line::from(""),
        heading("Mouse"),
        key("Click tree item", "Select (drill into level)"),
        key("Double-click", "Expand/collapse"),
        key("Hover tree/map", "Highlight geometry"),
        key("Click on map", "Select hovered feature"),
        key("Scroll panels", "Scroll tree/properties"),
        key("Drag dividers", "Resize panels"),
        Line::from(""),
        heading("Map Colors"),
        color(CLR_POINT, "Magenta", "Point"),
        color(CLR_MULTI_POINT, "Light magenta", "MultiPoint"),
        color(CLR_LINE, "Cyan", "LineString"),
        color(CLR_MULTI_LINE, "Light cyan", "MultiLineString"),
        color(CLR_POLYGON, "Blue", "Polygon (outer ring, CCW)"),
        color(CLR_MULTI_POLYGON, "Light blue", "MultiPolygon"),
        color(CLR_INNER_RING, "Red", "Inner ring (hole, CW)"),
        color(CLR_BAD_WINDING, "Light red", "Non-standard winding"),
        color(CLR_EXTENT, "Dark gray", "Tile extent boundary"),
        Line::from(""),
        heading("Selection Colors"),
        color(CLR_SELECTED, "Yellow", "Selected feature/part"),
        color(CLR_HOVERED, "White", "Hovered feature"),
        color(CLR_INNER_RING_SEL, "Salmon", "Inner ring (selected)"),
        color(CLR_DIMMED, "Dark gray", "Sibling parts (dimmed)"),
    ]
}
