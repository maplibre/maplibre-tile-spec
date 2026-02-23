use std::collections::HashSet;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::prelude::{Line, Span, Style};
use ratatui::widgets::{Cell, Paragraph, Row, Table, Wrap};
use size_format::SizeFormatterSI;

use crate::ls::{LsRow, NA, na, row_cells};
use crate::ui::rendering::map;
use crate::ui::state::App;
use crate::ui::{
    CLR_DIMMED, CLR_HINT, CLR_HOVERED, STYLE_BOLD, STYLE_LABEL, STYLE_SELECTED, block_with_title,
    collect_extensions, collect_file_algorithms, collect_file_geometries,
};

pub fn render_tile_preview_panel(f: &mut Frame<'_>, area: Rect, app: &App) {
    if let Some(ref fc) = app.preview_fc {
        map::render_tile_preview(f, area, fc, app.preview_extent);
    } else {
        let msg = if app
            .get_selected_file()
            .and_then(|r| {
                app.preview_load_requested
                    .as_ref()
                    .filter(|p| p.as_path() == r.path())
            })
            .is_some()
        {
            "Loading…"
        } else {
            "Select a tile file (.mlt / .mvt) to preview"
        };
        f.render_widget(
            Paragraph::new(Line::from(msg)).block(block_with_title("Tile Preview")),
            area,
        );
    }
}

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
        .map(|&i| Row::new(row_cells(&app.files[i]).map(Cell::from)))
        .collect();

    let file_w = app
        .files
        .iter()
        .map(|r| row_cells(r)[0].len())
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
    let total = app.files.len();
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
    let exts = collect_extensions(&app.files);
    let geoms = collect_file_geometries(&app.files);
    let algos = collect_file_algorithms(&app.files);
    let has_any =
        !app.ext_filters.is_empty() || !app.geom_filters.is_empty() || !app.algo_filters.is_empty();

    let selected_mlt = app.get_selected_file();
    let sel_ext: Option<String> = selected_mlt
        .and_then(|r| r.path().extension().and_then(|e| e.to_str()))
        .map(str::to_lowercase);
    let sel_info = selected_mlt.and_then(|r| match r {
        LsRow::Info(_, i) => Some(i),
        _ => None,
    });

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

        let sel_geoms: HashSet<_> = sel_info
            .map(|i| i.geometries.iter().copied().collect())
            .unwrap_or_default();
        for g in &geoms {
            lines.push(Line::from(Span::styled(
                format!("  {}{g}", check(app.geom_filters.contains(g))),
                present_style(sel_geoms.contains(g)),
            )));
        }
    }
    if !algos.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Algorithms:", STYLE_BOLD)));

        let sel_algos: HashSet<_> = sel_info
            .map(|i| i.algorithms.iter().copied().collect())
            .unwrap_or_default();
        for a in &algos {
            lines.push(Line::from(Span::styled(
                format!("  {}{a}", check(app.algo_filters.contains(a))),
                present_style(sel_algos.contains(a)),
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

pub fn render_file_info_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let info = app.get_selected_file().and_then(|r| match r {
        LsRow::Info(_, i) => Some(i),
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
            row("File", info.path.clone(), ""),
            row("Size", sz(info.size), "raw MLT file size"),
            row(
                "Encoding",
                na(info.encoding_pct.map(|p| format!("{p:.1}%"))),
                "MLT / (data + metadata)",
            ),
            row("Data", na(info.data_size.map(&sz)), "decoded payload size"),
            row(
                "Metadata",
                match (info.meta_size, info.meta_pct) {
                    (Some(m), Some(p)) => format!("{} ({:.1}% of data)", sz(m), p),
                    _ => NA.to_string(),
                },
                "encoding overhead",
            ),
            row("Layers", info.layers.to_string(), "tile layer count"),
            row(
                "Features",
                info.features.to_string(),
                "total across all layers",
            ),
            row(
                "Streams",
                na(info.streams.map(|n| n.to_string())),
                "encoded data streams",
            ),
            row(
                "Geometries",
                info.geometries_display(),
                "geometry types present",
            ),
            row(
                "Algorithms",
                info.algorithms_display(),
                "compression methods",
            ),
        ]
    } else if app.filtered_file_indices.is_empty() && !app.files.is_empty() {
        vec![
            Line::from("No files match the current filters."),
            Line::from(""),
            Line::from(Span::styled("[Reset filters]", STYLE_SELECTED)),
        ]
    } else {
        vec![Line::from("Select a file to view details")]
    };

    let inner = area.height.saturating_sub(2) as usize;
    let max = u16::try_from(lines.len().saturating_sub(inner)).unwrap_or(0);
    app.file_info_scroll = app.file_info_scroll.min(max);

    let para = Paragraph::new(lines)
        .block(block_with_title("File Info"))
        .wrap(Wrap { trim: false })
        .scroll((app.file_info_scroll, 0));
    f.render_widget(para, area);
}
