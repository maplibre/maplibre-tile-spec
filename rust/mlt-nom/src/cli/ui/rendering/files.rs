use std::collections::HashSet;

use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::prelude::{Color, Line, Modifier, Span, Style};
use ratatui::widgets::{Cell, Paragraph, Row, Table, Wrap};

use crate::cli::ls::{LsRow, MltFileInfo, row_cells};
use crate::cli::ui::{App, block_with_title, collect_file_values, geom_abbrev_to_full};

pub fn render_file_browser(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    app.file_table_area = Some(area);
    app.file_table_inner_height = area.height.saturating_sub(3) as usize;

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let right = ratatui::layout::Alignment::Right;
    let header = Row::new(vec![
        Cell::from("File"),
        Cell::from(Line::from("Size").alignment(right)),
        Cell::from(Line::from("Enc %").alignment(right)),
        Cell::from(Line::from("Layers").alignment(right)),
        Cell::from(Line::from("Features").alignment(right)),
    ])
    .style(bold);

    let rows: Vec<Row> = app
        .filtered_file_indices
        .iter()
        .map(|&i| Row::new(row_cells(&app.mlt_files[i].1).map(Cell::from)))
        .collect();

    let file_col_width = app
        .mlt_files
        .iter()
        .map(|(_, r)| row_cells(r)[0].len())
        .max()
        .unwrap_or(4)
        .max(4);

    let widths = [
        Constraint::Length(u16::try_from(file_col_width).unwrap_or_default().min(200)),
        Constraint::Length(8),
        Constraint::Length(7),
        Constraint::Length(6),
        Constraint::Length(10),
    ];
    app.file_table_widths = Some(widths);

    let sort_hint = if app.data_loaded() {
        " Click header to sort"
    } else {
        ""
    };
    let filtered = app.filtered_file_indices.len();
    let total = app.mlt_files.len();
    let count = if filtered < total {
        format!("{filtered}/{total}")
    } else {
        total.to_string()
    };
    let title = format!("MLT Files ({count} found) - ↑/↓ navigate, Enter open, q quit{sort_hint}");
    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .block(block_with_title(title))
        .row_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    f.render_stateful_widget(table, area, &mut app.file_list_state);
}

pub fn render_file_filter_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let geoms = collect_file_values(&app.mlt_files, MltFileInfo::geometries);
    let algos = collect_file_values(&app.mlt_files, MltFileInfo::algorithms);
    let has_any = !app.geom_filters.is_empty() || !app.algo_filters.is_empty();

    let sel_info = app
        .selected_file_real_index()
        .and_then(|i| app.mlt_files.get(i))
        .and_then(|(_, r)| match r {
            LsRow::Info(i) => Some(i),
            _ => None,
        });
    let sel_geoms: HashSet<&str> = sel_info
        .map(|i| i.geometries().split(',').map(str::trim).collect())
        .unwrap_or_default();
    let sel_algos: HashSet<&str> = sel_info
        .map(|i| i.algorithms().split(',').map(str::trim).collect())
        .unwrap_or_default();

    let mut lines: Vec<Line<'static>> = Vec::new();
    let reset_style = if has_any {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(Span::styled("[Reset filters]", reset_style)));
    lines.push(Line::from(""));

    let check = |active: bool| if active { "[x] " } else { "[ ] " };
    let item_style = |present: bool| {
        if present {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    };

    if !geoms.is_empty() {
        lines.push(Line::from(Span::styled(
            "Geometry Types:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for g in &geoms {
            lines.push(Line::from(Span::styled(
                format!(
                    "  {}{}",
                    check(app.geom_filters.contains(g)),
                    geom_abbrev_to_full(g)
                ),
                item_style(sel_geoms.contains(g.as_str())),
            )));
        }
    }
    if !algos.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Algorithms:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
        for a in &algos {
            lines.push(Line::from(Span::styled(
                format!("  {}{a}", check(app.algo_filters.contains(a))),
                item_style(sel_algos.contains(a.as_str())),
            )));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from("(loading…)"));
    }

    let inner_height = area.height.saturating_sub(2);
    let max_scroll = u16::try_from(lines.len().saturating_sub(inner_height as usize)).unwrap_or(0);
    app.filter_scroll = app.filter_scroll.min(max_scroll);

    let para = Paragraph::new(lines)
        .block(block_with_title("Filter (click to toggle)"))
        .scroll((app.filter_scroll, 0));
    f.render_widget(para, area);
}

pub fn render_file_info_panel(f: &mut Frame<'_>, area: Rect, app: &App) {
    let info = app
        .selected_file_real_index()
        .and_then(|i| app.mlt_files.get(i))
        .and_then(|(_, r)| match r {
            LsRow::Info(i) => Some(i),
            _ => None,
        });

    let lines: Vec<Line<'static>> = if let Some(info) = info {
        let fmt_size = |n: usize| format!("{:.1}B", size_format::SizeFormatterSI::new(n as u64));
        let label = Style::default().fg(Color::Cyan);
        let hint = Style::default().fg(Color::DarkGray);
        let row = |name: &str, val: String, desc: &str| -> Line<'static> {
            let mut spans = vec![Span::styled(format!("{name}: "), label), Span::raw(val)];
            if !desc.is_empty() {
                spans.push(Span::styled(format!("  {desc}"), hint));
            }
            Line::from(spans)
        };
        vec![
            row("File", info.path().to_string(), ""),
            row("Size", fmt_size(info.size()), "raw MLT file size"),
            row(
                "Encoding",
                format!("{:.1}%", info.encoding_pct()),
                "MLT / (data + metadata)",
            ),
            row("Data", fmt_size(info.data_size()), "decoded payload size"),
            row(
                "Metadata",
                format!(
                    "{} ({:.1}% of data)",
                    fmt_size(info.meta_size()),
                    info.meta_pct()
                ),
                "encoding overhead",
            ),
            row("Layers", info.layers().to_string(), "tile layer count"),
            row(
                "Features",
                info.features().to_string(),
                "total across all layers",
            ),
            row(
                "Streams",
                info.streams().to_string(),
                "encoded data streams",
            ),
            row(
                "Geometries",
                info.geometries().to_string(),
                "geometry types present",
            ),
            row(
                "Algorithms",
                info.algorithms().to_string(),
                "compression methods",
            ),
        ]
    } else if app.filtered_file_indices.is_empty() && !app.mlt_files.is_empty() {
        vec![
            Line::from("No files match the current filters."),
            Line::from(""),
            Line::from(Span::styled(
                "[Reset filters]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
        ]
    } else {
        vec![Line::from("Select a file to view details")]
    };

    let para = Paragraph::new(lines)
        .block(block_with_title("File Info"))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}
