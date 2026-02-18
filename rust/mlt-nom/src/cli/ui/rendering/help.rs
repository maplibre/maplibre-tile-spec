use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::cli::ui::block_with_title;
use crate::cli::ui::state::{App, ViewMode};

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
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);
    let inner_height = height.saturating_sub(2);
    let max_scroll = u16::try_from(lines.len().saturating_sub(inner_height as usize)).unwrap_or(0);
    app.help_scroll = app.help_scroll.min(max_scroll);
    f.render_widget(ratatui::widgets::Clear, popup);
    let para = Paragraph::new(lines)
        .block(block_with_title(
            "Help (\u{2191}/\u{2193}/scroll to navigate, any other key to close)",
        ))
        .scroll((app.help_scroll, 0));
    f.render_widget(para, popup);
}

fn help_heading(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))
}

fn help_key(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {key:<20}"), Style::default().fg(Color::Cyan)),
        Span::raw(desc.to_string()),
    ])
}

fn help_color(color: Color, label: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{label:<20}"), Style::default().fg(color)),
        Span::raw(desc.to_string()),
    ])
}

fn help_file_browser() -> Vec<Line<'static>> {
    vec![
        help_heading("Keyboard"),
        help_key("?  h  F1", "Toggle this help"),
        help_key("q  Ctrl+c", "Quit"),
        help_key("Up/Down  j/k", "Navigate file list"),
        help_key("PageUp/PageDown", "Scroll by page"),
        help_key("Home/End", "Jump to first/last"),
        help_key("Enter", "Open selected MLT file"),
        help_key("Esc", "Quit"),
        Line::from(""),
        help_heading("Mouse"),
        help_key("Click row", "Select file"),
        help_key("Double-click row", "Open file"),
        help_key("Click header", "Sort by column"),
        help_key("Scroll", "Navigate file list"),
        help_key("Drag divider", "Resize panels"),
        Line::from(""),
        help_heading("Filter Panel"),
        help_key("Click checkbox", "Toggle geometry/algorithm filter"),
        help_key("Click [Reset]", "Clear all filters"),
    ]
}

fn help_layer_overview() -> Vec<Line<'static>> {
    vec![
        help_heading("Keyboard"),
        help_key("?  h  F1", "Toggle this help"),
        help_key("q  Ctrl+c", "Quit"),
        help_key("Esc", "Back to file browser"),
        help_key("Up/Down  j/k", "Navigate feature tree"),
        help_key("PageUp/PageDown", "Scroll by page"),
        help_key("Home/End", "Jump to first/last"),
        help_key("Enter", "Expand/collapse layer or feature"),
        help_key("+  =  Right", "Expand selected node"),
        help_key("-", "Collapse (or jump to parent)"),
        help_key("*", "Expand/collapse all layers"),
        help_key("Left", "Jump to parent node"),
        help_key("Ctrl+h / Ctrl+l", "Resize left/right split"),
        help_key("Shift+J / Shift+K", "Resize top/bottom split"),
        Line::from(""),
        help_heading("Mouse"),
        help_key("Click tree item", "Select (drill into level)"),
        help_key("Double-click", "Expand/collapse"),
        help_key("Hover tree/map", "Highlight geometry"),
        help_key("Click on map", "Select hovered feature"),
        help_key("Scroll panels", "Scroll tree/properties"),
        help_key("Drag dividers", "Resize panels"),
        Line::from(""),
        help_heading("Map Colors"),
        help_color(Color::Magenta, "Magenta", "Point"),
        help_color(Color::LightMagenta, "Light magenta", "MultiPoint"),
        help_color(Color::Cyan, "Cyan", "LineString"),
        help_color(Color::LightCyan, "Light cyan", "MultiLineString"),
        help_color(Color::Blue, "Blue", "Polygon (outer ring, CCW)"),
        help_color(Color::LightBlue, "Light blue", "MultiPolygon"),
        help_color(Color::Red, "Red", "Inner ring (hole, CW)"),
        help_color(Color::LightRed, "Light red", "Non-standard winding"),
        help_color(Color::DarkGray, "Dark gray", "Tile extent boundary"),
        Line::from(""),
        help_heading("Selection Colors"),
        help_color(Color::Yellow, "Yellow", "Selected feature/part"),
        help_color(Color::White, "White", "Hovered feature"),
        help_color(Color::Rgb(255, 150, 120), "Salmon", "Inner ring (selected)"),
        help_color(Color::DarkGray, "Dark gray", "Sibling parts (dimmed)"),
    ]
}
