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
use mlt_core::StatType::{DecodedDataSize, DecodedMetaSize, FeatureCount};
use mlt_core::mvt::mvt_to_feature_collection;
use mlt_core::v01::{
    DictionaryType, Geometry, GeometryType, LengthType, LogicalDecoder, OffsetType,
    PhysicalDecoder, PhysicalStreamType, Stream,
};
use mlt_core::{Analyze as _, parse_layers};
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
    /// Paths to tile files (.mlt, .mvt, .pbf) or directories
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Filter by file extension (e.g. mlt, mvt, pbf). Can be specified multiple times.
    #[arg(short, long)]
    extension: Vec<String>,

    /// Disable recursive directory traversal
    #[arg(long)]
    no_recursive: bool,

    /// Output format (table or JSON)
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

/// Column index for file table sorting in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSortColumn {
    File,
    Size,
    EncPct,
    Layers,
    Features,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct MltFileInfo {
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

impl MltFileInfo {
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
    #[must_use]
    pub fn size(&self) -> usize {
        self.size
    }
    #[must_use]
    pub fn encoding_pct(&self) -> f64 {
        self.encoding_pct
    }
    #[must_use]
    pub fn data_size(&self) -> usize {
        self.data_size
    }
    #[must_use]
    pub fn meta_size(&self) -> usize {
        self.meta_size
    }
    #[must_use]
    pub fn meta_pct(&self) -> f64 {
        self.meta_pct
    }
    #[must_use]
    pub fn layers(&self) -> usize {
        self.layers
    }
    #[must_use]
    pub fn features(&self) -> usize {
        self.features
    }
    #[must_use]
    pub fn streams(&self) -> usize {
        self.streams
    }
    #[must_use]
    pub fn geometries(&self) -> &str {
        &self.geometries
    }
    #[must_use]
    pub fn algorithms(&self) -> &str {
        &self.algorithms
    }
}

#[derive(serde::Serialize, Clone)]
#[serde(untagged)]
pub enum LsRow {
    Info(MltFileInfo),
    Error {
        path: String,
        error: String,
    },
    /// Placeholder while analysis is in progress
    Loading {
        path: String,
    },
}

#[must_use]
pub fn relative_path(path: &Path, base_path: &Path) -> String {
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

    for path in &args.paths {
        let files = collect_tile_files(path, recursive, &args.extension)?;
        all_files.extend(files);
    }

    if all_files.is_empty() {
        eprintln!("No tile files found");
        return Ok(());
    }

    // Determine base path for relative path calculation
    // Use current directory if multiple paths or use the single path
    let base_path = if args.paths.len() == 1 {
        &args.paths[0]
    } else {
        Path::new(".")
    };

    let rows: Vec<_> = all_files
        .par_iter()
        .map(|path| match analyze_tile_file(path, base_path, false) {
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

/// Analyze tile files (MLT and MVT) and return rows (for reuse by UI).
/// Uses parallel iteration when rayon is enabled.
/// When `skip_gzip` is true, gzip size estimation is skipped for speed.
#[must_use]
pub fn analyze_tile_files(paths: &[PathBuf], base_path: &Path, skip_gzip: bool) -> Vec<LsRow> {
    paths
        .par_iter()
        .map(|path| match analyze_tile_file(path, base_path, skip_gzip) {
            Ok(info) => LsRow::Info(info),
            Err(e) => LsRow::Error {
                path: relative_path(path, base_path),
                error: e.to_string(),
            },
        })
        .collect()
}

/// Return cells for UI table display: [File, Size, Enc%, Layers, Features].
#[must_use]
pub fn row_cells(row: &LsRow) -> [String; 5] {
    let fmt_size = |n: usize| format!("{:.1}B", SizeFormatterSI::new(n as u64));
    match row {
        LsRow::Info(info) => [
            info.path().to_string(),
            format!("{:>8}", fmt_size(info.size())),
            format!("{:>6}", fmt_pct(info.encoding_pct())),
            format!("{:>6}", info.layers()),
            format!("{:>10}", info.features().separate_with_commas()),
        ],
        LsRow::Error { path, error } => [
            path.clone(),
            format!("ERROR: {error}"),
            String::new(),
            String::new(),
            String::new(),
        ],
        LsRow::Loading { path } => [
            path.clone(),
            "…".to_string(),
            "…".to_string(),
            "…".to_string(),
            "…".to_string(),
        ],
    }
}

pub(crate) fn is_tile_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some("mlt" | "mvt" | "pbf")
    )
}

pub(crate) fn is_mvt_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some("mvt" | "pbf")
    )
}

fn matches_extension_filter(path: &Path, extensions: &[String]) -> bool {
    let ext = path
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_lowercase);
    match ext {
        Some(ext) => extensions
            .iter()
            .any(|e| e.trim_start_matches('.').to_lowercase() == ext),
        None => false,
    }
}

fn collect_tile_files(path: &Path, recursive: bool, extensions: &[String]) -> Result<Vec<PathBuf>> {
    let matches_ext = |p: &Path| {
        if extensions.is_empty() {
            is_tile_extension(p)
        } else {
            matches_extension_filter(p, extensions)
        }
    };

    let mut files = Vec::new();

    if path.is_file() {
        if matches_ext(path) {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        collect_from_dir(path, &mut files, recursive, &matches_ext)?;
    }

    Ok(files)
}

fn collect_from_dir<F>(
    dir: &Path,
    files: &mut Vec<PathBuf>,
    recursive: bool,
    matches_ext: &F,
) -> Result<()>
where
    F: Fn(&Path) -> bool,
{
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            if matches_ext(&path) {
                files.push(path);
            }
        } else if recursive && path.is_dir() {
            collect_from_dir(&path, files, recursive, matches_ext)?;
        }
    }
    Ok(())
}

pub fn analyze_tile_file(path: &Path, base_path: &Path, skip_gzip: bool) -> Result<MltFileInfo> {
    let buffer = fs::read(path)?;
    let original_size = buffer.len();

    if is_mvt_extension(path) {
        return analyze_mvt_buffer(&buffer, original_size, path, base_path, skip_gzip);
    }

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

    let mut geometries = HashSet::new();
    let mut feature_count = 0;
    let mut data_size = 0;
    let mut meta_size = 0;

    for layer in &mut layers {
        layer.decode_all()?;
        if let Some(layer01) = layer.as_layer01() {
            data_size += layer01.collect_statistic(DecodedDataSize);
            meta_size += layer01.collect_statistic(DecodedMetaSize);
            feature_count += layer01.collect_statistic(FeatureCount);

            if let Geometry::Decoded(ref geom) = layer01.geometry {
                for &geom_type in &geom.vector_types {
                    geometries.insert(geom_type);
                }
            }
        }
    }

    let (gzipped_size, gzip_pct) = if skip_gzip {
        (0, 0.0)
    } else {
        let gz = estimate_gzip_size(&buffer)?;
        (gz, percent(gz, original_size))
    };

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
        gzip_pct,
        layers: layers.len(),
        features: feature_count,
        streams: stream_count,
        algorithms: algorithms_str,
        geometries: geometries_str,
    })
}

fn analyze_mvt_buffer(
    buffer: &[u8],
    original_size: usize,
    path: &Path,
    base_path: &Path,
    skip_gzip: bool,
) -> Result<MltFileInfo> {
    use mlt_core::geojson::Geometry as GjGeom;

    let fc = mvt_to_feature_collection(buffer.to_vec())?;

    let mut layer_names = HashSet::new();
    let mut geom_types = HashSet::new();
    for feat in &fc.features {
        // FIXME: we shouldn't use "magical" properties to pass values around
        if let Some(name) = feat.properties.get("_layer").and_then(|v| v.as_str()) {
            layer_names.insert(name.to_string());
        }
        geom_types.insert(match &feat.geometry {
            GjGeom::Point(_) => "Pt",
            GjGeom::MultiPoint(_) => "MPt",
            GjGeom::LineString(_) => "Line",
            GjGeom::MultiLineString(_) => "MLine",
            GjGeom::Polygon(_) => "Poly",
            GjGeom::MultiPolygon(_) => "MPoly",
        });
    }

    let (gzipped_size, gzip_pct) = if skip_gzip {
        (0, 0.0)
    } else {
        let gz = estimate_gzip_size(buffer)?;
        (gz, percent(gz, original_size))
    };

    let mut sorted_geoms: Vec<_> = geom_types.into_iter().collect();
    sorted_geoms.sort_unstable();

    Ok(MltFileInfo {
        path: relative_path(path, base_path),
        size: original_size,
        encoding_pct: 0.0,
        data_size: 0,
        meta_size: 0,
        meta_pct: 0.0,
        gzipped_size,
        gzip_pct,
        layers: layer_names.len(),
        features: fc.features.len(),
        streams: 0,
        algorithms: "protobuf".to_string(),
        geometries: sorted_geoms.join(","),
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

    let infos: Vec<&MltFileInfo> = rows
        .iter()
        .filter_map(|r| match r {
            LsRow::Info(info) => Some(info),
            LsRow::Error { .. } | LsRow::Loading { .. } => None,
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
                    info.path().to_string(),
                    fmt_size(info.size()),
                    fmt_pct(info.encoding_pct()),
                    fmt_size(info.data_size),
                    fmt_size(info.meta_size),
                    fmt_pct(info.meta_pct),
                    fmt_size(info.gzipped_size),
                    fmt_pct(info.gzip_pct),
                    info.layers().separate_with_commas(),
                    info.features().separate_with_commas(),
                    info.streams.separate_with_commas(),
                    info.geometries().to_string(),
                ];
                if extended {
                    data_row.push(info.algorithms().to_string());
                }
                builder.push_record(data_row);
            }
            LsRow::Error { path, error } => {
                let mut data_row = vec![path.clone(), format!("ERROR: {error}")];
                data_row.resize(num_cols, String::new());
                builder.push_record(data_row);
                error_table_rows.push(i + 1);
            }
            LsRow::Loading { path } => {
                let mut data_row = vec![path.clone(), "Loading…".to_string()];
                data_row.resize(num_cols, String::new());
                builder.push_record(data_row);
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

fn fmt_pct(v: f64) -> String {
    if v.abs() >= 10.0 {
        format!("{v:.0}%")
    } else if v.abs() >= 1.0 {
        format!("{v:.1}%")
    } else {
        format!("{v:.2}%")
    }
}
