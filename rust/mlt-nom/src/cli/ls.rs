use std::collections::HashSet;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Args, ValueEnum};
use flate2::Compression;
use flate2::write::GzEncoder;
use mlt_nom::v01::{
    DictionaryType, Geometry, GeometryType, LengthType, LogicalDecoder, OffsetType,
    PhysicalDecoder, PhysicalStreamType, Stream,
};
use mlt_nom::{Analyze as _, StatType, parse_layers};
#[cfg(feature = "rayon")]
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use size_format::SizeFormatterSI;
use tabled::Table;
use tabled::builder::Builder;
use tabled::settings::object::{Cell, Columns};
use tabled::settings::span::ColumnSpan;
use tabled::settings::style::HorizontalLine;
use tabled::settings::{Alignment, Style};
use thousands::Separable as _;

#[derive(Args)]
pub struct LsArgs {
    /// Paths to .mlt files or directories
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Disable recursive directory traversal
    #[arg(long)]
    no_recursive: bool,

    /// Output format (table or json)
    #[arg(short, long, default_value = "table", value_enum)]
    format: LsFormat,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, ValueEnum)]
pub enum LsFormat {
    /// Table output with aligned columns
    #[default]
    Table,
    /// Extended table output (includes algorithm details)
    ExtendedTable,
    /// JSON output
    Json,
}

/// Compression reduction: `(1 - compressed/original) * 100`.
/// Returns 0 if `original` is 0.
#[expect(clippy::cast_precision_loss)]
fn percent(compressed: usize, original: usize) -> f64 {
    if original > 0 {
        (1.0 - compressed as f64 / original as f64) * 100.0
    } else {
        0.0
    }
}

#[expect(clippy::cast_precision_loss)]
fn percent_of(part: usize, whole: usize) -> f64 {
    if whole > 0 {
        (part as f64 / whole as f64) * 100.0
    } else {
        0.0
    }
}

#[derive(serde::Serialize, Debug)]
struct MltFileInfo {
    path: String,
    size: usize,
    encoding_pct: f64,
    data_size: usize,
    meta_size: usize,
    meta_pct: f64,
    gzipped_size: usize,
    gzip_pct: f64,
    layers: usize,
    features: usize,
    streams: usize,
    algorithms: String,
    geometries: String,
}

#[derive(serde::Serialize)]
#[serde(untagged)]
enum LsRow {
    Info(MltFileInfo),
    Error { path: String, error: String },
}

fn relative_path(path: &Path, base_path: &Path) -> String {
    if base_path.is_file() {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    } else {
        path.strip_prefix(base_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    }
}

pub fn ls(args: &LsArgs) -> Result<()> {
    let recursive = !args.no_recursive;
    let mut all_files = Vec::new();

    // Collect files from all provided paths
    for path in &args.paths {
        let files = collect_mlt_files(path, recursive)?;
        all_files.extend(files);
    }

    if all_files.is_empty() {
        eprintln!("No .mlt files found");
        return Ok(());
    }

    // Determine base path for relative path calculation
    // Use current directory if multiple paths or use the single path
    let base_path = if args.paths.len() == 1 {
        &args.paths[0]
    } else {
        Path::new(".")
    };

    // Process files in parallel if rayon is enabled, otherwise sequentially
    #[cfg(feature = "rayon")]
    let all_files = all_files.par_iter();
    #[cfg(not(feature = "rayon"))]
    let all_files = all_files.iter();

    let rows: Vec<_> = all_files
        .map(|path| match analyze_mlt_file(path, base_path) {
            Ok(info) => LsRow::Info(info),
            Err(e) => LsRow::Error {
                path: relative_path(path, base_path),
                error: e.to_string(),
            },
        })
        .collect();

    match args.format {
        LsFormat::Table => print_table(&rows, false),
        LsFormat::ExtendedTable => print_table(&rows, true),
        LsFormat::Json => println!("{}", serde_json::to_string_pretty(&rows)?),
    }

    Ok(())
}

fn collect_mlt_files(path: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if path.is_file() {
        if path.extension().and_then(OsStr::to_str) == Some("mlt") {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        collect_from_dir(path, &mut files, recursive)?;
    }

    Ok(files)
}

fn collect_from_dir(dir: &Path, files: &mut Vec<PathBuf>, recursive: bool) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            if path.extension().and_then(|s| s.to_str()) == Some("mlt") {
                files.push(path);
            }
        } else if recursive && path.is_dir() {
            collect_from_dir(&path, files, recursive)?;
        }
    }
    Ok(())
}

fn analyze_mlt_file(path: &Path, base_path: &Path) -> Result<MltFileInfo> {
    let buffer = fs::read(path)?;
    let original_size = buffer.len();
    let mut layers = parse_layers(&buffer)?;

    let mut stream_count = 0;
    let mut algorithms: HashSet<StreamStat> = HashSet::new();
    for layer in &layers {
        if let Some(layer01) = layer.as_layer01() {
            layer01.for_each_stream(&mut |stream| {
                stream_count += 1;
                collect_stream_info(stream, &mut algorithms);
            });
        }
    }

    // Now decode to get feature counts and geometry types
    let mut geometries = HashSet::new();
    let mut feature_count = 0;
    let mut data_size = 0;
    let mut meta_size = 0;

    for layer in &mut layers {
        layer.decode_all()?;
        if let Some(layer01) = layer.as_layer01() {
            data_size += layer01.decoded(StatType::PayloadDataSizeBytes);
            meta_size += layer01.decoded(StatType::MetadataOverheadBytes);
            feature_count += layer01.decoded(StatType::FeatureCount);

            if let Geometry::Decoded(ref geom) = layer01.geometry {
                for &geom_type in &geom.vector_types {
                    geometries.insert(geom_type);
                }
            }
        }
    }

    // Calculate gzip size
    let gzipped_size = estimate_gzip_size(&buffer)?;

    // Format compression and geometry lists with abbreviations
    let geometries_str = format_geometries(geometries);
    let algorithms_str = format_algorithms(algorithms);

    let rel_path = relative_path(path, base_path);

    Ok(MltFileInfo {
        path: rel_path,
        size: original_size,
        encoding_pct: percent(original_size, data_size + meta_size),
        data_size,
        meta_size,
        meta_pct: percent_of(meta_size, data_size),
        gzipped_size,
        gzip_pct: percent(gzipped_size, original_size),
        layers: layers.len(),
        features: feature_count,
        streams: stream_count,
        algorithms: algorithms_str,
        geometries: geometries_str,
    })
}

type StreamStat = (PhysicalStreamType, PhysicalDecoder, StatLogicalDecoder);

/// Mirrors [`LogicalDecoder`] without associated metadata values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum StatLogicalDecoder {
    None,
    Delta,
    DeltaRle,
    ComponentwiseDelta,
    Rle,
    Morton,
    PseudoDecimal,
}

impl From<LogicalDecoder> for StatLogicalDecoder {
    fn from(ld: LogicalDecoder) -> Self {
        match ld {
            LogicalDecoder::None => Self::None,
            LogicalDecoder::Delta => Self::Delta,
            LogicalDecoder::DeltaRle(_) => Self::DeltaRle,
            LogicalDecoder::ComponentwiseDelta => Self::ComponentwiseDelta,
            LogicalDecoder::Rle(_) => Self::Rle,
            LogicalDecoder::Morton(_) => Self::Morton,
            LogicalDecoder::PseudoDecimal => Self::PseudoDecimal,
        }
    }
}

fn collect_stream_info(stream: &Stream, algo: &mut HashSet<StreamStat>) {
    algo.insert((
        stream.meta.physical_type,
        stream.meta.physical_decoder,
        StatLogicalDecoder::from(stream.meta.logical_decoder),
    ));
}

fn estimate_gzip_size(data: &[u8]) -> Result<usize> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;
    Ok(compressed.len())
}

fn format_algorithms(algorithms: HashSet<StreamStat>) -> String {
    let mut sorted: Vec<_> = algorithms.into_iter().collect();
    sorted.sort();
    sorted
        .into_iter()
        .map(|(phys_type, phys_dec, log_dec)| {
            let phys_type = match phys_type {
                PhysicalStreamType::Present => "Present",
                PhysicalStreamType::Data(v) => match v {
                    DictionaryType::None => "RawData",
                    DictionaryType::Vertex => "Vertex",
                    DictionaryType::Single => "Single",
                    DictionaryType::Shared => "Shared",
                    DictionaryType::Morton => "Morton",
                    DictionaryType::Fsst => "Fsst",
                },
                PhysicalStreamType::Offset(v) => match v {
                    OffsetType::Vertex => "VertexOffset",
                    OffsetType::Index => "IndexOffset",
                    OffsetType::String => "StringOffset",
                    OffsetType::Key => "KeyOffset",
                },
                PhysicalStreamType::Length(v) => match v {
                    LengthType::VarBinary => "VarBinaryLen",
                    LengthType::Geometries => "GeomLen",
                    LengthType::Parts => "PartsLen",
                    LengthType::Rings => "RingsLen",
                    LengthType::Triangles => "TrianglesLen",
                    LengthType::Symbol => "SymbolLen",
                    LengthType::Dictionary => "DictLen",
                },
            };
            let phys_dec = match phys_dec {
                PhysicalDecoder::None => "",
                PhysicalDecoder::FastPFOR => "FastPFOR",
                PhysicalDecoder::VarInt => "VarInt",
                PhysicalDecoder::Alp => "Alp",
            };
            let log_dec = match log_dec {
                StatLogicalDecoder::None => "",
                StatLogicalDecoder::Delta => "Delta",
                StatLogicalDecoder::DeltaRle => "DeltaRle",
                StatLogicalDecoder::Rle => "Rle",
                StatLogicalDecoder::ComponentwiseDelta => "CwDelta",
                StatLogicalDecoder::Morton => "Morton",
                StatLogicalDecoder::PseudoDecimal => "PseudoDec",
            };
            let mut val = phys_type.to_owned();
            if !phys_dec.is_empty() {
                let _ = write!(val, "-{phys_dec}");
            }
            if !log_dec.is_empty() {
                let _ = write!(val, "-{log_dec}");
            }
            val
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn format_geometries(geometries: HashSet<GeometryType>) -> String {
    let mut sorted: Vec<_> = geometries.into_iter().collect();
    sorted.sort();
    sorted
        .into_iter()
        .map(|g| match g {
            GeometryType::Point => "Pt",
            GeometryType::LineString => "Line",
            GeometryType::Polygon => "Poly",
            GeometryType::MultiPoint => "MPt",
            GeometryType::MultiLineString => "MLine",
            GeometryType::MultiPolygon => "MPoly",
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn print_table(rows: &[LsRow], extended: bool) {
    let fmt_size = |n: usize| format!("{:.1}B", SizeFormatterSI::new(n as u64));
    let fmt_pct = |v: f64| {
        if v.abs() >= 10.0 {
            format!("{v:.0}%")
        } else if v.abs() >= 1.0 {
            format!("{v:.1}%")
        } else {
            format!("{v:.2}%")
        }
    };

    let infos: Vec<&MltFileInfo> = rows
        .iter()
        .filter_map(|r| match r {
            LsRow::Info(info) => Some(info),
            LsRow::Error { .. } => None,
        })
        .collect();
    let has_total = infos.len() > 1;
    let mut error_table_rows = Vec::new();
    let mut builder = Builder::default();

    let mut header = vec![
        "File",
        "Size",
        "Enc %",
        "Decoded",
        "Meta",
        "Meta %",
        "Gzipped",
        "Gz %",
        "Layer",
        "Feature",
        "Stream",
        "Geometry Types",
    ];
    if extended {
        header.push("Algorithms");
    }
    let num_cols = header.len();
    builder.push_record(header);

    for (i, row) in rows.iter().enumerate() {
        match row {
            LsRow::Info(info) => {
                let mut data_row = vec![
                    info.path.clone(),
                    fmt_size(info.size),
                    fmt_pct(info.encoding_pct),
                    fmt_size(info.data_size),
                    fmt_size(info.meta_size),
                    fmt_pct(info.meta_pct),
                    fmt_size(info.gzipped_size),
                    fmt_pct(info.gzip_pct),
                    info.layers.separate_with_commas(),
                    info.features.separate_with_commas(),
                    info.streams.separate_with_commas(),
                    info.geometries.clone(),
                ];
                if extended {
                    data_row.push(info.algorithms.clone());
                }
                builder.push_record(data_row);
            }
            LsRow::Error { path, error } => {
                let mut data_row = vec![path.clone(), format!("ERROR: {error}")];
                data_row.resize(num_cols, String::new());
                builder.push_record(data_row);
                error_table_rows.push(i + 1);
            }
        }
    }

    if has_total {
        let total_size: usize = infos.iter().map(|i| i.size).sum();
        let total_data: usize = infos.iter().map(|i| i.data_size).sum();
        let total_meta: usize = infos.iter().map(|i| i.meta_size).sum();
        let total_gzipped: usize = infos.iter().map(|i| i.gzipped_size).sum();
        let total_layers: usize = infos.iter().map(|i| i.layers).sum();
        let total_features: usize = infos.iter().map(|i| i.features).sum();
        let total_streams: usize = infos.iter().map(|i| i.streams).sum();

        let mut row = vec![
            "TOTAL".to_string(),
            fmt_size(total_size),
            fmt_pct(percent(total_size, total_data + total_meta)),
            fmt_size(total_data),
            fmt_size(total_meta),
            fmt_pct(percent_of(total_meta, total_data)),
            fmt_size(total_gzipped),
            fmt_pct(percent(total_gzipped, total_size)),
            total_layers.separate_with_commas(),
            total_features.separate_with_commas(),
            total_streams.separate_with_commas(),
            String::new(),
        ];
        if extended {
            row.push(String::new());
        }
        builder.push_record(row);
    }

    let header_line = HorizontalLine::new('-').intersection('+');
    let mut table = Table::from(builder);

    #[expect(clippy::cast_possible_wrap)]
    let col_span = ColumnSpan::new((num_cols - 1) as isize);
    for &row_idx in &error_table_rows {
        table.modify(Cell::new(row_idx, 1), col_span);
    }

    if has_total {
        let total_row = rows.len() + 1;
        table.with(
            Style::empty()
                .vertical('|')
                .horizontals([(1, header_line), (total_row, header_line)]),
        );
    } else {
        table.with(Style::empty().vertical('|').horizontals([(1, header_line)]));
    }
    table.modify(Columns::new(1..10), Alignment::right());
    for &row_idx in &error_table_rows {
        table.modify(Cell::new(row_idx, 1), Alignment::left());
    }

    println!("{table}");
}
