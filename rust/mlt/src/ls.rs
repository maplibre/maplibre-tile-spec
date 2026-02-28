use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::string::ToString;

use anyhow::Result;
use clap::{Args, ValueEnum};
use flate2::Compression;
use flate2::write::GzEncoder;
use globset::{GlobSet, GlobSetBuilder};
use mlt_core::StatType::{DecodedDataSize, DecodedMetaSize, FeatureCount};
use mlt_core::geojson::{FeatureCollection, Geom32};
use mlt_core::mvt::mvt_to_feature_collection;
use mlt_core::v01::{
    DictionaryType, Geometry, GeometryType, LengthType, LogicalEncoding, OffsetType,
    PhysicalEncoding, Stream, StreamType,
};
use mlt_core::{Analyze as _, parse_layers};
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use serde::Serialize;
use serde_json::Value as JsonValue;
use size_format::SizeFormatterSI;
use tabled::Table;
use tabled::builder::Builder;
use tabled::settings::object::{Cell, Columns};
use tabled::settings::span::ColumnSpan;
use tabled::settings::style::HorizontalLine;
use tabled::settings::{Alignment, Style};
use thousands::Separable as _;

#[derive(Debug, Args)]
pub struct LsArgs {
    /// Paths to tile files (.mlt, .mvt, .pbf) or directories
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Filter by file extension (e.g. mlt, mvt, pbf). Can be specified multiple times.
    #[arg(short = 'e', long)]
    extension: Vec<String>,

    /// Exclude paths matching the given glob (e.g. "**/fixtures/**", "**/*.pbf"). Can be specified multiple times.
    #[arg(short = 'E', long = "exclude")]
    exclude: Vec<String>,

    /// Disable recursive directory traversal
    #[arg(long)]
    no_recursive: bool,

    /// Level of detail to show (can be specified multiple times for more details)
    #[arg(short, long, value_enum, default_values = ["basic", "gzip"])]
    details: Vec<Detail>,

    /// Output format (table or JSON)
    #[arg(short, long, default_value = "table", value_enum)]
    format: LsFormat,

    /// Validate tile files against JSON validation files in the same directory (with .json extension)
    #[arg(long)]
    validate_to_json: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Detail {
    /// Show basic statistics: file size, encoding %, layers, features
    Basic,
    /// Show all available statistics
    All,
    /// Show gzip size estimation and compression ratio
    #[clap(name = "gzip")]
    GZip,
    /// Show stream/encoding algorithms used (Algorithms column)
    Algorithms,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LsFlags {
    pub gzip: bool,
    pub algorithms: bool,
    pub validate: bool,
}

impl From<&LsArgs> for LsFlags {
    fn from(args: &LsArgs) -> Self {
        use Detail::{Algorithms, All, GZip};
        let details = args.details.as_slice();
        Self {
            gzip: details.contains(&GZip) || details.contains(&All),
            algorithms: details.contains(&Algorithms) || details.contains(&All),
            validate: args.validate_to_json,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, ValueEnum)]
pub enum LsFormat {
    /// Table output with aligned columns
    Table,
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

/// Algorithm description for a file (MLT stream combo or protobuf for MVT).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileAlgorithm {
    Mlt(StreamType, PhysicalEncoding, StatLogicalCodec),
    Mvt,
}

impl std::fmt::Display for FileAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mvt => write!(f, "Protobuf"),
            Self::Mlt(phys_type, physical, logical) => {
                let phys_type = match phys_type {
                    StreamType::Present => "Present",
                    StreamType::Data(v) => match v {
                        DictionaryType::None => "RawData",
                        DictionaryType::Vertex => "Vertex",
                        DictionaryType::Single => "Single",
                        DictionaryType::Shared => "Shared",
                        DictionaryType::Morton => "Morton",
                        DictionaryType::Fsst => "Fsst",
                    },
                    StreamType::Offset(v) => match v {
                        OffsetType::Vertex => "VertexOffset",
                        OffsetType::Index => "IndexOffset",
                        OffsetType::String => "StringOffset",
                        OffsetType::Key => "KeyOffset",
                    },
                    StreamType::Length(v) => match v {
                        LengthType::VarBinary => "VarBinaryLen",
                        LengthType::Geometries => "GeomLen",
                        LengthType::Parts => "PartsLen",
                        LengthType::Rings => "RingsLen",
                        LengthType::Triangles => "TrianglesLen",
                        LengthType::Symbol => "SymbolLen",
                        LengthType::Dictionary => "DictLen",
                    },
                };
                let physical = match physical {
                    PhysicalEncoding::None => "",
                    PhysicalEncoding::FastPFOR => "FastPFOR",
                    PhysicalEncoding::VarInt => "VarInt",
                    PhysicalEncoding::Alp => "Alp",
                };
                let logical = match logical {
                    StatLogicalCodec::None => "",
                    StatLogicalCodec::Delta => "Delta",
                    StatLogicalCodec::DeltaRle => "DeltaRle",
                    StatLogicalCodec::Rle => "Rle",
                    StatLogicalCodec::ComponentwiseDelta => "CwDelta",
                    StatLogicalCodec::Morton => "Morton",
                    StatLogicalCodec::MortonDelta => "MortonDelta",
                    StatLogicalCodec::MortonRle => "MortonRle",
                    StatLogicalCodec::PseudoDecimal => "PseudoDec",
                };
                write!(f, "{phys_type}")?;
                if !physical.is_empty() {
                    write!(f, "-{physical}")?;
                }
                if !logical.is_empty() {
                    write!(f, "-{logical}")?;
                }
                Ok(())
            }
        }
    }
}

impl Serialize for FileAlgorithm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Dash shown when a numeric column is not applicable (e.g. MVT has no Enc %).
pub const NA: &str = "—";

#[must_use]
pub fn na(v: Option<String>) -> String {
    v.unwrap_or_else(|| NA.to_string())
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MltFileInfo {
    pub path: String,
    pub size: usize,
    pub encoding_pct: Option<f64>,
    pub data_size: Option<usize>,
    pub meta_size: Option<usize>,
    pub meta_pct: Option<f64>,
    pub gzipped_size: Option<usize>,
    pub gzip_pct: Option<f64>,
    pub layers: usize,
    pub features: usize,
    pub streams: Option<usize>,
    pub algorithms: HashSet<FileAlgorithm>,
    pub geometries: HashSet<GeometryType>,
    pub matches_json: Option<bool>,
}

impl MltFileInfo {}

impl MltFileInfo {
    #[must_use]
    pub fn geometries_display(&self) -> String {
        geometries_display(&self.geometries)
    }
    #[must_use]
    pub fn algorithms_display(&self) -> String {
        algorithms_display(&self.algorithms)
    }
}

#[derive(serde::Serialize, Clone)]
#[serde(untagged)]
#[expect(clippy::large_enum_variant)]
pub enum LsRow {
    Info {
        path: PathBuf,
        info: MltFileInfo,
    },
    Error {
        path: PathBuf,
        size: Option<usize>,
        error: String,
    },
    /// Placeholder while analysis is in progress
    Loading {
        path: PathBuf,
    },
}

impl LsRow {
    /// Path for this row (file path, or path that failed/loading).
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            LsRow::Info { path, .. } | LsRow::Error { path, .. } | LsRow::Loading { path } => {
                path.as_path()
            }
        }
    }
}

/// True if the path string contains glob metacharacters `"*?[{"`.
fn has_glob_metachars(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains('*') || s.contains('?') || s.contains('[') || s.contains('{')
}

/// Expand path arguments: if a path contains glob metacharacters, expand it to matching paths;
/// otherwise use the path as-is. Directories are left as-is so `collect_tile_files` can recurse into them.
fn expand_path_args(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for path in paths {
        if has_glob_metachars(path) {
            for entry in glob::glob(path.to_string_lossy().as_ref())? {
                out.push(entry?);
            }
        } else {
            out.push(path.clone());
        }
    }
    Ok(out)
}

/// Build a `GlobSet` from patterns; returns None if patterns is empty.
fn build_exclude_set(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        builder.add(globset::Glob::new(p)?);
    }
    Ok(Some(builder.build()?))
}

/// List tile files with statistics.
/// Returns `true` if all files were valid, `false` if any file had an error, or no files.
pub fn ls(args: &LsArgs) -> Result<bool> {
    let flags = LsFlags::from(args);
    let mut all_files = Vec::new();

    // Expand path arguments as globs when they contain *?[{; directories are left as-is and handled below.
    let expanded_paths = expand_path_args(&args.paths)?;
    let exclude = build_exclude_set(&args.exclude)?;

    for path in &expanded_paths {
        let files = collect_tile_files(path, args, exclude.as_ref())?;
        all_files.extend(files);
    }

    if all_files.is_empty() {
        eprintln!("No tile files found");
        return Ok(false);
    }

    let base_path = if args.paths.len() == 1 && !has_glob_metachars(&args.paths[0]) {
        &args.paths[0]
    } else {
        Path::new(".")
    };

    let result = analyze_tile_files(all_files.as_slice(), base_path, flags);
    match args.format {
        LsFormat::Table => print_table(&result, flags),
        LsFormat::Json => println!("{}", serde_json::to_string_pretty(&result)?),
    }

    Ok(result.iter().all(|r| match r {
        LsRow::Info {
            info: MltFileInfo { matches_json, .. },
            ..
        } => matches_json.unwrap_or(true),
        _ => false,
    }))
}

/// Analyze tile files (MLT and MVT) and return rows (for reuse by UI).
#[must_use]
pub fn analyze_tile_files(paths: &[PathBuf], base_path: &Path, flags: LsFlags) -> Vec<LsRow> {
    paths
        .par_iter()
        .map(|path| match analyze_tile_file(path, base_path, flags) {
            Ok(info) => LsRow::Info {
                path: path.clone(),
                info,
            },
            Err(e) => LsRow::Error {
                path: path.clone(),
                error: e.to_string(),
                size: fs::metadata(path)
                    .ok()
                    .and_then(|m| usize::try_from(m.len()).ok()),
            },
        })
        .collect()
}

/// Return cells for UI table display: [File, Size, Enc%, Layers, Features].
#[must_use]
pub fn row_cells(row: &LsRow) -> [String; 5] {
    let fmt_size = |n: usize| format!("{:.1}B", SizeFormatterSI::new(n as u64));
    match row {
        LsRow::Info { info, .. } => [
            info.path.clone(),
            format!("{:>8}", fmt_size(info.size)),
            format!("{:>6}", na(info.encoding_pct.map(fmt_pct))),
            format!("{:>6}", info.layers),
            format!("{:>10}", info.features.separate_with_commas()),
        ],
        LsRow::Error {
            path,
            error: _,
            size,
        } => [
            path.display().to_string(),
            size.map_or_else(String::new, |n| {
                format!("{:>8}", format!("{:.1}B", SizeFormatterSI::new(n as u64)))
            }),
            String::new(),
            String::new(),
            String::new(),
        ],
        LsRow::Loading { path } => [
            path.display().to_string(),
            "…".to_string(),
            "…".to_string(),
            "…".to_string(),
            "…".to_string(),
        ],
    }
}

/// Path string for UI display; when `base` is given, returns path relative to base (same as Info row display).
#[must_use]
pub fn path_display(path: &Path, base: Option<&Path>) -> String {
    match base {
        None => path.display().to_string(),
        Some(b) if b.is_file() => path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
        Some(b) => path.strip_prefix(b).map_or_else(
            |_| path.display().to_string(),
            |p| p.to_string_lossy().to_string(),
        ),
    }
}

/// Six-column cells for UI table: [File, Size, Enc %, Layers, Features, Notes]. Uses `path_display(path, base)` for the file column. Notes column is error message for Error rows, empty otherwise.
#[must_use]
pub fn row_cells_6(row: &LsRow, base: Option<&Path>) -> [String; 6] {
    let cells5 = row_cells(row);
    let file_col = path_display(row.path(), base);
    let notes = match row {
        LsRow::Error { error, .. } => error.clone(),
        LsRow::Info { .. } | LsRow::Loading { .. } => String::new(),
    };
    [
        file_col,
        cells5[1].clone(),
        cells5[2].clone(),
        cells5[3].clone(),
        cells5[4].clone(),
        notes,
    ]
}

pub(crate) fn is_tile_extension(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some("mlt" | "mvt" | "pbf")
    )
}

pub(crate) fn is_mlt_extension(path: &Path) -> bool {
    matches!(path.extension().and_then(OsStr::to_str), Some("mlt"))
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

fn collect_tile_files(
    path: &Path,
    args: &LsArgs,
    exclude_set: Option<&GlobSet>,
) -> Result<Vec<PathBuf>> {
    let matches_ext = |p: &Path| {
        if args.extension.is_empty() {
            is_tile_extension(p)
        } else {
            matches_extension_filter(p, &args.extension)
        }
    };
    let excluded = |p: &Path| exclude_set.is_some_and(|s| s.is_match(p));

    let mut files = Vec::new();
    if path.is_dir() {
        collect_from_dir(
            path,
            &mut files,
            !args.no_recursive,
            &matches_ext,
            exclude_set,
        )?;
    } else if path.is_file() && !excluded(path) && matches_ext(path) {
        files.push(path.to_path_buf());
    }

    Ok(files)
}

fn collect_from_dir<F>(
    dir: &Path,
    files: &mut Vec<PathBuf>,
    recursive: bool,
    matches_ext: &F,
    exclude_set: Option<&GlobSet>,
) -> Result<()>
where
    F: Fn(&Path) -> bool,
{
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            if !exclude_set.is_some_and(|s| s.is_match(&path)) && matches_ext(&path) {
                files.push(path);
            }
        } else if recursive && path.is_dir() && !exclude_set.is_some_and(|s| s.is_match(&path)) {
            collect_from_dir(&path, files, recursive, matches_ext, exclude_set)?;
        }
    }
    Ok(())
}

pub fn analyze_tile_file(path: &Path, base_path: &Path, flags: LsFlags) -> Result<MltFileInfo> {
    let buffer = fs::read(path)?;
    let mut info = if is_mlt_extension(path) {
        analyze_mlt_buffer(&buffer, path, flags)?
    } else {
        analyze_mvt_buffer(&buffer)?
    };
    info.path = if base_path.is_file() {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    } else {
        path.strip_prefix(base_path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    };
    if flags.gzip {
        let gzip_size = estimate_gzip_size(&buffer)?;
        info.gzipped_size = Some(gzip_size);
        info.gzip_pct = Some(percent(gzip_size, buffer.len()));
    }
    Ok(info)
}

pub fn analyze_mlt_buffer(buffer: &[u8], path: &Path, flags: LsFlags) -> Result<MltFileInfo> {
    let mut layers = parse_layers(buffer)?;

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

    let matches_json = if flags.validate {
        let json_path = path.with_extension("json");
        if json_path.is_file() {
            let expected: FeatureCollection =
                serde_json::from_str(&fs::read_to_string(&json_path)?)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            let actual = FeatureCollection::from_layers(&layers)?;
            let expected_val = normalize_tiny_floats(serde_json::to_value(&expected)?);
            let actual_val = normalize_tiny_floats(serde_json::to_value(&actual)?);
            Some(json_values_equal(&expected_val, &actual_val))
        } else {
            Some(false)
        }
    } else {
        None
    };

    let algorithms: HashSet<FileAlgorithm> = algorithms
        .into_iter()
        .map(|(a, b, c)| FileAlgorithm::Mlt(a, b, c))
        .collect();

    Ok(MltFileInfo {
        size: buffer.len(),
        encoding_pct: Some(percent(buffer.len(), data_size + meta_size)),
        data_size: Some(data_size),
        meta_size: Some(meta_size),
        meta_pct: Some(percent_of(meta_size, data_size)),
        layers: layers.len(),
        features: feature_count,
        streams: Some(stream_count),
        algorithms,
        geometries,
        matches_json,
        ..MltFileInfo::default()
    })
}

/// Compare two JSON values for equality. Numbers are compared with float tolerance so that
/// f32 round-trip (e.g. 3.14 vs 3.140000104904175) and Java minimal decimal (e.g. 3.4028235e+38)
/// match the Rust decoder output.
fn json_values_equal(a: &JsonValue, b: &JsonValue) -> bool {
    match (a, b) {
        (JsonValue::Number(na), JsonValue::Number(nb)) if na.is_f64() && nb.is_f64() => {
            let na = na.as_f64().expect("f64");
            let nb = nb.as_f64().expect("f64");
            assert!(
                !na.is_nan() && !nb.is_nan(),
                "unexpected non-finite numbers"
            );
            let abs_diff = (na - nb).abs();
            let max_abs = na.abs().max(nb.abs()).max(1.0);
            abs_diff <= f64::from(f32::EPSILON) * max_abs * 2.0
        }
        (JsonValue::Array(aa), JsonValue::Array(ab)) => {
            aa.len() == ab.len()
                && aa
                    .iter()
                    .zip(ab.iter())
                    .all(|(x, y)| json_values_equal(x, y))
        }
        (JsonValue::Object(ao), JsonValue::Object(bo)) => {
            ao.len() == bo.len()
                && ao
                    .iter()
                    .all(|(k, v)| bo.get(k).is_some_and(|w| json_values_equal(v, w)))
        }
        _ => a == b,
    }
}

/// Replace tiny float values (e.g. `1e-40`) with `0.0` to handle codec precision issues.
fn normalize_tiny_floats(value: JsonValue) -> JsonValue {
    match value {
        JsonValue::Number(ref n) => {
            let eps = f64::from(f32::EPSILON);
            if let Some(f) = n.as_f64()
                && f.is_finite()
                && f.abs() < eps
            {
                JsonValue::from(0.0)
            } else {
                value
            }
        }
        JsonValue::Array(arr) => {
            JsonValue::Array(arr.into_iter().map(normalize_tiny_floats).collect())
        }
        JsonValue::Object(obj) => JsonValue::Object(
            obj.into_iter()
                .map(|(k, v)| (k, normalize_tiny_floats(v)))
                .collect(),
        ),
        v => v,
    }
}

fn analyze_mvt_buffer(buffer: &[u8]) -> Result<MltFileInfo> {
    let fc = mvt_to_feature_collection(buffer.to_vec())?;

    let mut layer_names = HashSet::new();
    let mut geometries = HashSet::new();
    for feat in &fc.features {
        // FIXME: we shouldn't use "magical" properties to pass values around
        if let Some(name) = feat.properties.get("_layer").and_then(|v| v.as_str()) {
            layer_names.insert(name.to_string());
        }
        if let Some(gt) = geometry_type_from_geom32(&feat.geometry) {
            geometries.insert(gt);
        }
    }

    Ok(MltFileInfo {
        size: buffer.len(),
        layers: layer_names.len(),
        features: fc.features.len(),
        algorithms: std::iter::once(FileAlgorithm::Mvt).collect(),
        geometries,
        ..MltFileInfo::default()
    })
}

fn geometry_type_from_geom32(geom: &Geom32) -> Option<GeometryType> {
    Some(match geom {
        Geom32::Point(_) => GeometryType::Point,
        Geom32::MultiPoint(_) => GeometryType::MultiPoint,
        Geom32::LineString(_) => GeometryType::LineString,
        Geom32::MultiLineString(_) => GeometryType::MultiLineString,
        Geom32::Polygon(_) => GeometryType::Polygon,
        Geom32::MultiPolygon(_) => GeometryType::MultiPolygon,
        Geom32::Line(_) | Geom32::GeometryCollection(_) | Geom32::Rect(_) | Geom32::Triangle(_) => {
            return None;
        }
    })
}

type StreamStat = (StreamType, PhysicalEncoding, StatLogicalCodec);

/// Mirrors [`LogicalEncoding`] without associated metadata values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StatLogicalCodec {
    None,
    Delta,
    DeltaRle,
    ComponentwiseDelta,
    Rle,
    Morton,
    MortonDelta,
    MortonRle,
    PseudoDecimal,
}

impl From<LogicalEncoding> for StatLogicalCodec {
    fn from(ld: LogicalEncoding) -> Self {
        match ld {
            LogicalEncoding::None => Self::None,
            LogicalEncoding::Delta => Self::Delta,
            LogicalEncoding::DeltaRle(_) => Self::DeltaRle,
            LogicalEncoding::ComponentwiseDelta => Self::ComponentwiseDelta,
            LogicalEncoding::Rle(_) => Self::Rle,
            LogicalEncoding::Morton(_) => Self::Morton,
            LogicalEncoding::MortonDelta(_) => Self::MortonDelta,
            LogicalEncoding::MortonRle(_) => Self::MortonRle,
            LogicalEncoding::PseudoDecimal => Self::PseudoDecimal,
        }
    }
}

fn collect_stream_info(stream: &Stream, algo: &mut HashSet<StreamStat>) {
    algo.insert((
        stream.meta.stream_type,
        stream.meta.physical_encoding,
        StatLogicalCodec::from(stream.meta.logical_encoding),
    ));
}

fn estimate_gzip_size(data: &[u8]) -> Result<usize> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    let compressed = encoder.finish()?;
    Ok(compressed.len())
}

fn geometries_display(geometries: &HashSet<GeometryType>) -> String {
    let abbrev = |g: GeometryType| match g {
        GeometryType::Point => "Pt",
        GeometryType::LineString => "Line",
        GeometryType::Polygon => "Poly",
        GeometryType::MultiPoint => "MPt",
        GeometryType::MultiLineString => "MLine",
        GeometryType::MultiPolygon => "MPoly",
    };
    let mut v: Vec<GeometryType> = geometries.iter().copied().collect();
    v.sort_unstable();
    v.iter().map(|g| abbrev(*g)).collect::<Vec<_>>().join(",")
}

fn algorithms_display(algorithms: &HashSet<FileAlgorithm>) -> String {
    let mut v: Vec<_> = algorithms.iter().map(ToString::to_string).collect();
    v.sort_unstable();
    v.join(",")
}

fn print_table(rows: &[LsRow], flags: LsFlags) {
    let fmt_size = |n: usize| format!("{:.1}B", SizeFormatterSI::new(n as u64));

    let infos: Vec<&MltFileInfo> = rows
        .iter()
        .filter_map(|r| match r {
            LsRow::Info { info, .. } => Some(info),
            LsRow::Error { .. } | LsRow::Loading { .. } => None,
        })
        .collect();
    let has_total = infos.len() > 1;
    let mut error_table_rows = Vec::new();
    let mut builder = Builder::default();

    let mut header = vec!["File", "Size", "Enc %", "Decoded", "Meta", "Meta %"];
    if flags.gzip {
        header.push("Gzipped");
        header.push("Gz %");
    }
    header.extend(["Layer", "Feature", "Stream", "Geometry Types"]);
    if flags.validate {
        header.push("JSON");
    }
    if flags.algorithms {
        header.push("Algorithms");
    }
    let num_cols = header.len();
    builder.push_record(header);

    for (i, row) in rows.iter().enumerate() {
        match row {
            LsRow::Info { info, .. } => {
                let mut data_row = vec![
                    info.path.clone(),
                    fmt_size(info.size),
                    na(info.encoding_pct.map(fmt_pct)),
                    na(info.data_size.map(fmt_size)),
                    na(info.meta_size.map(fmt_size)),
                    na(info.meta_pct.map(fmt_pct)),
                ];
                if flags.gzip {
                    data_row.push(na(info.gzipped_size.map(fmt_size)));
                    data_row.push(na(info.gzip_pct.map(fmt_pct)));
                }
                data_row.extend([
                    info.layers.separate_with_commas(),
                    info.features.separate_with_commas(),
                    na(info.streams.map(|n| n.separate_with_commas())),
                    info.geometries_display(),
                ]);
                if flags.validate {
                    data_row.push(match info.matches_json {
                        Some(true) => "✓".to_string(),
                        Some(false) => "✗".to_string(),
                        None => NA.to_string(),
                    });
                }
                if flags.algorithms {
                    data_row.push(info.algorithms_display());
                }
                builder.push_record(data_row);
            }
            LsRow::Error { path, error, size } => {
                let size_str = size.map_or_else(String::new, &fmt_size);
                let mut data_row = vec![
                    path.display().to_string(),
                    size_str,
                    format!("ERROR: {error}"),
                ];
                data_row.resize(num_cols, String::new());
                builder.push_record(data_row);
                error_table_rows.push(i + 1);
            }
            LsRow::Loading { path } => {
                let mut data_row = vec![path.display().to_string(), "Loading…".to_string()];
                data_row.resize(num_cols, String::new());
                builder.push_record(data_row);
            }
        }
    }

    if has_total {
        let total_size: usize = infos.iter().map(|i| i.size).sum();
        let total_data: Option<usize> = infos
            .iter()
            .try_fold(0usize, |acc, i| i.data_size.map(|d| acc + d));
        let total_meta: Option<usize> = infos
            .iter()
            .try_fold(0usize, |acc, i| i.meta_size.map(|m| acc + m));
        let total_gzipped: usize = infos.iter().filter_map(|i| i.gzipped_size).sum();
        let total_layers: usize = infos.iter().map(|i| i.layers).sum();
        let total_features: usize = infos.iter().map(|i| i.features).sum();
        let total_streams: Option<usize> = infos
            .iter()
            .try_fold(0usize, |acc, i| i.streams.map(|s| acc + s));

        let (enc_pct, decoded, meta, meta_pct) = match (total_data, total_meta) {
            (Some(d), Some(m)) => (
                fmt_pct(percent(total_size, d + m)),
                fmt_size(d),
                fmt_size(m),
                fmt_pct(percent_of(m, d)),
            ),
            _ => (
                NA.to_string(),
                NA.to_string(),
                NA.to_string(),
                NA.to_string(),
            ),
        };
        let mut row = vec![
            "TOTAL".to_string(),
            fmt_size(total_size),
            enc_pct,
            decoded,
            meta,
            meta_pct,
        ];
        if flags.gzip {
            let has_any_gzip = infos.iter().any(|i| i.gzipped_size.is_some());
            let gzip_size_str = if has_any_gzip {
                fmt_size(total_gzipped)
            } else {
                NA.to_string()
            };
            let gzip_pct_str = if has_any_gzip {
                fmt_pct(percent(total_gzipped, total_size))
            } else {
                NA.to_string()
            };
            row.push(gzip_size_str);
            row.push(gzip_pct_str);
        }
        row.extend([
            total_layers.separate_with_commas(),
            total_features.separate_with_commas(),
            na(total_streams.map(|s| s.separate_with_commas())),
            String::new(),
        ]);
        if flags.validate {
            row.push(String::new());
        }
        if flags.algorithms {
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
    // File - left aligned, size..stream (9-11) right, two more left
    table.modify(
        Columns::new(1..9 + if flags.gzip { 2 } else { 0 }),
        Alignment::right(),
    );
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
