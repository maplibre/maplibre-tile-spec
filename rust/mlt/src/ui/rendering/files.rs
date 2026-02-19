use std::collections::HashSet;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::prelude::{Line, Span, Style};
use ratatui::widgets::{Cell, Paragraph, Row, Table, Wrap};
use size_format::SizeFormatterSI;

use crate::ls::{LsRow, MltFileInfo, row_cells};
use crate::ui::state::App;
use crate::ui::{
    CLR_DIMMED, CLR_HINT, CLR_HOVERED, STYLE_BOLD, STYLE_LABEL, STYLE_SELECTED, block_with_title,
    collect_extensions, collect_file_values, geom_abbrev_to_full,
};

pub fn render_file_browser(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    app.file_table_area = Some(area);
    app.file_table_inner_height = area.height.saturating_sub(3) as usize;

    let header = Row::new(vec![
        Cell::from("File"),
        Cell::from(Line::from("Size").alignment(Alignment::Right)),
        Cell::from(Line::from("Enc %").alignment(Alignment::Right)),
        Cell::from(Line::from("Layers").alignment(Alignment::Right)),
        Cell::from(Line::from("Features").alignment(Alignment::Right)),
    ])
    .style(STYLE_BOLD);

    let rows: Vec<Row> = app
        .filtered_file_indices
        .iter()
        .map(|&i| Row::new(row_cells(&app.mlt_files[i].1).map(Cell::from)))
        .collect();

    let file_w = app
        .mlt_files
        .iter()
        .map(|(_, r)| row_cells(r)[0].len())
        .max()
        .unwrap_or(4)
        .max(4);

    let widths = [
        Constraint::Length(u16::try_from(file_w).unwrap_or_default().min(200)),
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
    let title =
        format!("MLT Files ({count} found) - ↑/↓ navigate, Enter open, h help, q quit{sort_hint}");
    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .block(block_with_title(title))
        .row_highlight_style(STYLE_SELECTED)
        .highlight_symbol(">> ");
    f.render_stateful_widget(table, area, &mut app.file_list_state);
}

pub fn render_file_filter_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let exts = collect_extensions(&app.mlt_files);
    let geoms = collect_file_values(&app.mlt_files, MltFileInfo::geometries);
    let algos = collect_file_values(&app.mlt_files, MltFileInfo::algorithms);
    let has_any =
        !app.ext_filters.is_empty() || !app.geom_filters.is_empty() || !app.algo_filters.is_empty();

    let sel_ext: Option<String> = app
        .selected_file_real_index()
        .and_then(|i| app.mlt_files.get(i))
        .and_then(|(p, _)| p.extension().and_then(|e| e.to_str()))
        .map(str::to_lowercase);
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
        STYLE_SELECTED
    } else {
        Style::default().fg(CLR_DIMMED)
    };
    lines.push(Line::from(Span::styled("[Reset filters]", reset_style)));
    lines.push(Line::from(""));

    let check = |on: bool| if on { "[x] " } else { "[ ] " };
    let present_style =
        |yes: bool| -> Style { Style::default().fg(if yes { CLR_HOVERED } else { CLR_DIMMED }) };

    if !exts.is_empty() {
        lines.push(Line::from(Span::styled("Extensions:", STYLE_BOLD)));
        for ext in &exts {
            lines.push(Line::from(Span::styled(
                format!("  {}{ext}", check(app.ext_filters.contains(ext))),
                present_style(sel_ext.as_deref() == Some(ext.as_str())),
            )));
        }
    }
    if !geoms.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Geometry Types:", STYLE_BOLD)));
        for g in &geoms {
            lines.push(Line::from(Span::styled(
                format!(
                    "  {}{}",
                    check(app.geom_filters.contains(g)),
                    geom_abbrev_to_full(g)
                ),
                present_style(sel_geoms.contains(g.as_str())),
            )));
        }
    }
    if !algos.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Algorithms:", STYLE_BOLD)));
        for a in &algos {
            lines.push(Line::from(Span::styled(
                format!("  {}{a}", check(app.algo_filters.contains(a))),
                present_style(sel_algos.contains(a.as_str())),
            )));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from("(loading…)"));
    }

    let inner = area.height.saturating_sub(2);
    let max = u16::try_from(lines.len().saturating_sub(inner as usize)).unwrap_or(0);
    app.filter_scroll = app.filter_scroll.min(max);

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
        let sz = |n: usize| format!("{:.1}B", SizeFormatterSI::new(n as u64));
        let row = |name: &str, val: String, desc: &str| -> Line<'static> {
            let mut spans = vec![
                Span::styled(format!("{name}: "), STYLE_LABEL),
                Span::raw(val),
            ];
            if !desc.is_empty() {
                spans.push(Span::styled(
                    format!("  {desc}"),
                    Style::default().fg(CLR_HINT),
                ));
            }
            Line::from(spans)
        };
        vec![
            row("File", info.path().to_string(), ""),
            row("Size", sz(info.size()), "raw MLT file size"),
            row(
                "Encoding",
                format!("{:.1}%", info.encoding_pct()),
                "MLT / (data + metadata)",
            ),
            row("Data", sz(info.data_size()), "decoded payload size"),
            row(
                "Metadata",
                format!("{} ({:.1}% of data)", sz(info.meta_size()), info.meta_pct()),
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
            Line::from(Span::styled("[Reset filters]", STYLE_SELECTED)),
        ]
    } else {
        vec![Line::from("Select a file to view details")]
    };

    let para = Paragraph::new(lines)
        .block(block_with_title("File Info"))
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}
