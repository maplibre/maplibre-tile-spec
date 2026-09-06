use std::collections::HashSet;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::prelude::{Line, Span, Style};
use ratatui::widgets::{Cell, HighlightSpacing, Paragraph, Row, Table, Wrap};
use size_format::SizeFormatterSI;
use usize_cast::{FromUsize as _, IntoUsize as _};

use crate::ls::{LsRow, NA, na, path_display, row_cells_6};
use crate::ui::rendering::map;
use crate::ui::rendering::scrollbar::{Scroll, render_vscrollbar, wrapped_rows};
use crate::ui::state::App;
use crate::ui::{
    CLR_DIMMED, CLR_HINT, CLR_HOVERED, STYLE_BOLD, STYLE_LABEL, STYLE_SELECTED, block_with_title,
    collect_extensions, collect_file_algorithms, collect_file_geometries,
};

pub fn render_tile_preview_panel(f: &mut Frame<'_>, area: Rect, app: &App) {
    if let Some(ref tile) = app.preview {
        map::render_tile_preview(f, area, &tile.fc, tile.extent);
        return;
    }
    let selected = app.get_selected_file().map(LsRow::path);
    let msg = if let Some(err) = app
        .preview_error
        .as_ref()
        .filter(|_| app.preview_tile_path.as_deref() == selected)
    {
        format!("Preview failed: {err}")
    } else if selected.is_some() && app.preview_load_requested.as_deref() == selected {
        "Loading…".into()
    } else {
        "Select a tile file (.mlt / .mvt) to preview".into()
    };
    f.render_widget(
        Paragraph::new(Line::from(msg))
            .wrap(Wrap { trim: true })
            .block(block_with_title("Tile Preview")),
        area,
    );
}

pub fn render_file_browser(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    app.file_table_area = Some(area);
    app.file_table_inner_height = area.height.saturating_sub(3).into_usize();

    let base = app.file_browser_base.as_deref();
    let file_w = app
        .files
        .iter()
        .map(|r| path_display(r.path(), base).len())
        .max()
        .unwrap_or(4)
        .max(4);

    let widths = [
        Constraint::Length(u16::try_from(file_w).unwrap_or_default().min(200)),
        Constraint::Length(8),
        Constraint::Length(7),
        Constraint::Length(6),
        Constraint::Length(10),
        Constraint::Min(0),
    ];
    app.file_table_widths = Some(widths);

    let header = Row::new(vec![
        Cell::from("File"),
        Cell::from(Line::from("Size").alignment(Alignment::Right)),
        Cell::from(Line::from("Enc %").alignment(Alignment::Right)),
        Cell::from(Line::from("Layers").alignment(Alignment::Right)),
        Cell::from(Line::from("Features").alignment(Alignment::Right)),
        Cell::from("Notes"),
    ])
    .style(STYLE_BOLD);

    let rows: Vec<Row> = app
        .filtered_file_indices
        .iter()
        .map(|&i| Row::new(row_cells_6(&app.files[i], base).map(Cell::from)))
        .collect();
    let row_count = rows.len();

    let status = app.scan_status();
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
    let progress = if status.scanning {
        ", scanning…".to_string()
    } else if status.pending > 0 {
        format!(", {} analyzing…", status.pending)
    } else {
        String::new()
    };
    let count = format!("{count} found{progress}");
    let title =
        format!("MLT Files ({count}) - ↑/↓ navigate, Enter open, h help, q quit{sort_hint}");
    let table = Table::new(rows, widths)
        .header(header)
        .column_spacing(1)
        .block(block_with_title(title))
        .row_highlight_style(STYLE_SELECTED)
        .highlight_symbol(">> ")
        .highlight_spacing(HighlightSpacing::Always);
    f.render_stateful_widget(table, area, &mut app.file_list_state);
    render_vscrollbar(
        f,
        area,
        Scroll {
            content: row_count,
            view: app.file_table_inner_height,
            pos: app.file_list_state.offset(),
        },
    );
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
        LsRow::Info { info, .. } => Some(info),
        LsRow::Error { .. } | LsRow::Loading { .. } => None,
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

    let inner = area.height.saturating_sub(2).into_usize();
    let content = lines.len();
    app.filter_scroll = app.filter_scroll.min(Scroll::max_pos(content, inner));
    let para = Paragraph::new(lines)
        .block(block_with_title("Filter (click to toggle)"))
        .scroll((app.filter_scroll, 0));
    f.render_widget(para, area);
    render_vscrollbar(f, area, Scroll::new(content, inner, app.filter_scroll));
}

pub fn render_file_info_panel(f: &mut Frame<'_>, area: Rect, app: &mut App) {
    let info = app.get_selected_file().and_then(|r| match r {
        LsRow::Info { info, .. } => Some(info),
        LsRow::Error { .. } | LsRow::Loading { .. } => None,
    });
    let base = app
        .file_browser_base
        .as_ref()
        .map_or_else(String::new, |p| p.display().to_string());
    let status = app.scan_status();

    let lines: Vec<Line<'static>> = if let Some(info) = info {
        let sz = |n: usize| format!("{:.1}B", SizeFormatterSI::new(u64::from_usize(n)));
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
    } else if app.files.is_empty() && status.scanning {
        vec![Line::from(format!("Scanning {base}…"))]
    } else if app.files.is_empty() {
        vec![Line::from(format!("No tile files found in {base}"))]
    } else if app.filtered_file_indices.is_empty() {
        vec![
            Line::from("No files match the current filters."),
            Line::from(""),
            Line::from(Span::styled("[Reset filters]", STYLE_SELECTED)),
        ]
    } else if matches!(app.get_selected_file(), Some(LsRow::Loading { .. })) {
        vec![Line::from("Analyzing…")]
    } else {
        vec![Line::from("Select a file to view details")]
    };

    let inner_h = area.height.saturating_sub(2).into_usize();
    let inner_w = area.width.saturating_sub(2).into_usize();
    let rows = wrapped_rows(&lines, inner_w);
    app.file_info_scroll = app.file_info_scroll.min(Scroll::max_pos(rows, inner_h));

    let para = Paragraph::new(lines)
        .block(block_with_title("File Info"))
        .wrap(Wrap { trim: false })
        .scroll((app.file_info_scroll, 0));
    f.render_widget(para, area);
    render_vscrollbar(f, area, Scroll::new(rows, inner_h, app.file_info_scroll));
}
