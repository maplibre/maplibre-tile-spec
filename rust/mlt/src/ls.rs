use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;
use std::string::ToString;

use anyhow::Result as AnyResult;
use clap::{Args, ValueEnum};
use flate2::Compression;
use flate2::write::GzEncoder;
use globset::{GlobSet, GlobSetBuilder};
use mlt_core::geojson::FeatureCollection;
use mlt_core::mvt::mvt_to_feature_collection;
use mlt_core::wire::StatType::{DecodedDataSize, DecodedMetaSize, FeatureCount};
use mlt_core::wire::{
    Analyze as _, BoolLogical, DictionaryType, FloatLogical, IntLogical, LengthType,
    LogicalEncoding, OffsetType, PhysicalEncoding, StreamMeta, StreamType, VertexLogical,
};
use mlt_core::{Decoder, GeometryType, Parser};
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use serde::Serialize;
use size_format::SizeFormatterSI;
use tabled::Table;
use tabled::builder::Builder;
use tabled::settings::object::{Cell, Columns};
use tabled::settings::span::ColumnSpan;
use tabled::settings::style::HorizontalLine;
use tabled::settings::{Alignment, Style};
use thousands::Separable as _;
use usize_cast::FromUsize as _;

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
                        #[cfg(feature = "unstable-v2")]
                        LengthType::Nested => "NestedLen",
                        // `mlt-core` resolves its features separately, so it may hand this
                        // build a nested length stream the match above cannot name.
                        #[cfg(not(feature = "unstable-v2"))]
                        #[allow(
                            unreachable_patterns,
                            reason = "reachable only when mlt-core has v2"
                        )]
                        _ => "NestedLen",
                    },
                };
                // `mlt-core` may carry v2-only encodings this build has no name for,
                // since its features are resolved separately from this crate's.
                #[cfg_attr(
                    not(feature = "unstable-v2"),
                    expect(
                        clippy::wildcard_enum_match_arm,
                        reason = "v2 encodings exist only when mlt-core has them"
                    )
                )]
                let physical = match physical {
                    PhysicalEncoding::None => "",
                    PhysicalEncoding::FastPFor(_) => "FastPFOR",
                    PhysicalEncoding::VarInt => "VarInt",
                    #[cfg(feature = "unstable-v2")]
                    PhysicalEncoding::BitPacked => "BitPacked",
                    #[cfg(not(feature = "unstable-v2"))]
                    #[allow(
                        unreachable_patterns,
                        reason = "reachable only when mlt-core has v2, but this crate doesn't"
                    )]
                    _ => "Unknown",
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
                    StatLogicalCodec::Dict => "Dict",
                    StatLogicalCodec::Alp => "ALP",
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
pub const NA: &str = "-";

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
    /// What the tile holds, as flags: the data types of its property and m-value
    /// columns, plus `m_values` when it has any.
    pub content: HashSet<&'static str>,
    pub streams: Option<usize>,
    pub algorithms: HashSet<FileAlgorithm>,
    pub geometries: HashSet<GeometryType>,
    pub matches_json: Option<bool>,
}

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
            Self::Info { path, .. } | Self::Error { path, .. } | Self::Loading { path } => {
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
fn expand_path_args(paths: &[PathBuf]) -> AnyResult<Vec<PathBuf>> {
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
fn build_exclude_set(patterns: &[String]) -> AnyResult<Option<GlobSet>> {
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
pub fn ls(args: &LsArgs) -> AnyResult<bool> {
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
        LsRow::Error { .. } | LsRow::Loading { .. } => false,
    }))
}

/// Analyze tile files (MLT and MVT) and return rows (for reuse by UI).
#[must_use]
pub fn analyze_tile_files(paths: &[PathBuf], base_path: &Path, flags: LsFlags) -> Vec<LsRow> {
    paths
        .par_iter()
        .map(|path| analyze_tile_row(path, base_path, flags))
        .collect()
}

/// Analyze one tile file into a row, turning failures into `LsRow::Error`.
#[must_use]
pub fn analyze_tile_row(path: &Path, base_path: &Path, flags: LsFlags) -> LsRow {
    match analyze_tile_file(path, base_path, flags) {
        Ok(info) => LsRow::Info {
            path: path.to_path_buf(),
            info,
        },
        Err(e) => LsRow::Error {
            path: path.to_path_buf(),
            error: e.to_string(),
            size: fs::metadata(path)
                .ok()
                .and_then(|m| usize::try_from(m.len()).ok()),
        },
    }
}

/// Return cells for UI table display: [File, Size, Enc%, Layers, Features].
#[must_use]
pub fn row_cells(row: &LsRow) -> [String; 5] {
    let fmt_size = |n: usize| format!("{:.1}B", SizeFormatterSI::new(u64::from_usize(n)));
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
                format!(
                    "{:>8}",
                    format!("{:.1}B", SizeFormatterSI::new(u64::from_usize(n)))
                )
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

pub(crate) fn is_mbt_extension(path: &Path) -> bool {
    matches!(path.extension().and_then(OsStr::to_str), Some("mbtiles"))
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
) -> AnyResult<Vec<PathBuf>> {
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
) -> AnyResult<()>
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

pub fn analyze_tile_file(path: &Path, base_path: &Path, flags: LsFlags) -> AnyResult<MltFileInfo> {
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

pub fn analyze_mlt_buffer(buffer: &[u8], path: &Path, flags: LsFlags) -> AnyResult<MltFileInfo> {
    let layers = Parser::default().parse_layers(buffer)?;

    let mut stream_count = 0;
    let mut algorithms: HashSet<StreamStat> = HashSet::new();
    for layer in &layers {
        if let Some(layer01) = layer.as_layer01() {
            layer01.for_each_stream(&mut |stream_meta| {
                stream_count += 1;
                collect_stream_info(stream_meta, &mut algorithms);
            });
        }
    }

    let layers = Decoder::default().decode_all(layers)?;

    let mut geometries = HashSet::new();
    let mut feature_count = 0;
    let mut data_size = 0;
    let mut meta_size = 0;
    let mut content: HashSet<&'static str> = HashSet::new();

    for layer in &layers {
        if let Some(layer01) = layer.as_layer01() {
            data_size += layer01.collect_statistic(DecodedDataSize);
            meta_size += layer01.collect_statistic(DecodedMetaSize);
            feature_count += layer01.collect_statistic(FeatureCount);
            for &geom_type in layer01.geometry_values().vector_types() {
                geometries.insert(geom_type);
            }
            for property in layer01.properties() {
                content.insert(property.kind().into());
            }
            // an m-value column's type counts the same as a property column's
            #[cfg(feature = "unstable-v2")]
            for column in layer01.m_values() {
                content.insert("m_values");
                content.insert(column.values().kind().into());
            }
        }
    }

    let layer_count = layers.len();
    let matches_json = if flags.validate {
        let json_path = path.with_extension("json");
        if json_path.is_file() {
            let expected = FeatureCollection::from_str(&fs::read_to_string(&json_path)?)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let actual = FeatureCollection::from_layers(layers)?;
            Some(actual.equals(&expected)?)
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
        layers: layer_count,
        features: feature_count,
        content,
        streams: Some(stream_count),
        algorithms,
        geometries,
        matches_json,
        ..MltFileInfo::default()
    })
}

fn analyze_mvt_buffer(buffer: &[u8]) -> AnyResult<MltFileInfo> {
    let fc = mvt_to_feature_collection(buffer)?;

    let mut layer_names = HashSet::new();
    let mut geometries = HashSet::new();
    for feat in &fc.features {
        // FIXME: we shouldn't use "magical" properties to pass values around
        if let Some(name) = feat.properties.get("_layer").and_then(|v| v.as_str()) {
            layer_names.insert(name.to_string());
        }
        if let Ok(gt) = GeometryType::try_from(&feat.geometry) {
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
    Dict,
    Alp,
}

impl From<LogicalEncoding> for StatLogicalCodec {
    fn from(ld: LogicalEncoding) -> Self {
        use LogicalEncoding as LE;
        match ld {
            LE::Int(IntLogical::None)
            | LE::Bool(BoolLogical::None)
            | LE::Float(FloatLogical::None)
            | LE::Vertex(VertexLogical::None) => Self::None,
            LE::Int(IntLogical::Delta) | LE::Vertex(VertexLogical::Delta) => Self::Delta,
            LE::Int(IntLogical::DeltaRle(_)) => Self::DeltaRle,
            LE::Vertex(VertexLogical::ComponentwiseDelta) => Self::ComponentwiseDelta,
            LE::Int(IntLogical::Rle(_)) | LE::Bool(BoolLogical::ByteRle(_)) => Self::Rle,
            LE::Vertex(VertexLogical::Morton(_)) => Self::Morton,
            LE::Vertex(VertexLogical::MortonDelta(_)) => Self::MortonDelta,
            LE::Vertex(VertexLogical::MortonRle(_)) => Self::MortonRle,
            LE::Float(FloatLogical::Dict) => Self::Dict,
            LE::Float(FloatLogical::Alp(_)) => Self::Alp,
        }
    }
}

fn collect_stream_info(meta: StreamMeta, algo: &mut HashSet<StreamStat>) {
    algo.insert((
        meta.stream_type,
        meta.encoding.physical,
        StatLogicalCodec::from(meta.encoding.logical),
    ));
}

fn estimate_gzip_size(data: &[u8]) -> AnyResult<usize> {
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
    println!("{}", render_table(rows, flags));
}

fn render_table(rows: &[LsRow], flags: LsFlags) -> String {
    let fmt_size = |n: usize| format!("{:.1}B", SizeFormatterSI::new(u64::from_usize(n)));

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
                if let Some(true) = info.matches_json
                    && flags.validate
                {
                    continue; // When validating, no need to show valid rows
                }
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
            LsRow::Loading { .. } => unreachable!("Loading?"),
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

    table.to_string()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mlt_info(path: &str) -> MltFileInfo {
        MltFileInfo {
            path: path.to_string(),
            size: 12_345,
            encoding_pct: Some(62.5),
            data_size: Some(32_000),
            meta_size: Some(900),
            meta_pct: Some(2.8),
            gzipped_size: Some(9_000),
            gzip_pct: Some(27.1),
            layers: 3,
            features: 1_234,
            content: ["str", "i32", "m_values"].into_iter().collect(),
            streams: Some(42),
            algorithms: std::iter::once(FileAlgorithm::Mlt(
                StreamType::Data(DictionaryType::None),
                PhysicalEncoding::VarInt,
                StatLogicalCodec::Delta,
            ))
            .collect(),
            geometries: [GeometryType::Polygon, GeometryType::Point]
                .into_iter()
                .collect(),
            matches_json: None,
        }
    }

    fn mvt_info(path: &str) -> MltFileInfo {
        MltFileInfo {
            path: path.to_string(),
            size: 54_321,
            layers: 2,
            features: 500,
            algorithms: std::iter::once(FileAlgorithm::Mvt).collect(),
            geometries: std::iter::once(GeometryType::LineString).collect(),
            ..MltFileInfo::default()
        }
    }

    fn info_row(info: MltFileInfo) -> LsRow {
        LsRow::Info {
            path: PathBuf::from(&info.path),
            info,
        }
    }

    fn error_row(path: &str, size: Option<usize>) -> LsRow {
        LsRow::Error {
            path: PathBuf::from(path),
            size,
            error: "unsupported version".to_string(),
        }
    }

    fn validated(mut info: MltFileInfo, matches: bool) -> MltFileInfo {
        info.matches_json = Some(matches);
        info
    }

    const GZIP: LsFlags = LsFlags {
        gzip: true,
        algorithms: false,
        validate: false,
    };
    const ALGORITHMS: LsFlags = LsFlags {
        gzip: false,
        algorithms: true,
        validate: false,
    };
    const VALIDATE: LsFlags = LsFlags {
        gzip: false,
        algorithms: false,
        validate: true,
    };

    #[test]
    fn a_file_algorithm_serializes_as_its_display_string() {
        let algorithms = [
            FileAlgorithm::Mvt,
            FileAlgorithm::Mlt(
                StreamType::Present,
                PhysicalEncoding::None,
                StatLogicalCodec::None,
            ),
            FileAlgorithm::Mlt(
                StreamType::Data(DictionaryType::Vertex),
                PhysicalEncoding::VarInt,
                StatLogicalCodec::DeltaRle,
            ),
            FileAlgorithm::Mlt(
                StreamType::Offset(OffsetType::String),
                PhysicalEncoding::None,
                StatLogicalCodec::Rle,
            ),
            FileAlgorithm::Mlt(
                StreamType::Length(LengthType::Rings),
                PhysicalEncoding::None,
                StatLogicalCodec::None,
            ),
        ];
        insta::assert_snapshot!(
            serde_json::to_string(&algorithms).expect("algorithms serialize"),
            @r#"["Protobuf","Present","Vertex-VarInt-DeltaRle","StringOffset-Rle","RingsLen"]"#
        );
    }

    #[test]
    fn path_display_without_a_base_keeps_the_whole_path() {
        insta::assert_snapshot!(path_display(Path::new("/tiles/omt/5_16_11.mlt"), None), @"/tiles/omt/5_16_11.mlt");
    }

    #[test]
    fn path_display_with_a_file_base_keeps_only_the_file_name() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        insta::assert_snapshot!(
            path_display(Path::new("/tiles/omt/5_16_11.mlt"), Some(&base)),
            @"5_16_11.mlt"
        );
    }

    #[test]
    fn path_display_with_a_directory_base_strips_the_prefix() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"));
        insta::assert_snapshot!(path_display(&base.join("Cargo.toml"), Some(base)), @"Cargo.toml");
    }

    #[test]
    fn path_display_with_an_unrelated_base_keeps_the_whole_path() {
        insta::assert_snapshot!(
            path_display(
                Path::new("/tiles/omt/5_16_11.mlt"),
                Some(Path::new("/no/such/base"))
            ),
            @"/tiles/omt/5_16_11.mlt"
        );
    }

    #[test]
    fn an_extension_filter_matches_case_insensitively() {
        assert!(matches_extension_filter(
            Path::new("/tiles/A.MLT"),
            &["mlt".to_string()]
        ));
    }

    #[test]
    fn an_extension_filter_ignores_a_leading_dot_in_the_pattern() {
        assert!(matches_extension_filter(
            Path::new("/tiles/a.pbf"),
            &[".mvt".to_string(), ".pbf".to_string()]
        ));
    }

    #[test]
    fn an_extension_filter_rejects_a_different_extension() {
        assert!(!matches_extension_filter(
            Path::new("/tiles/a.mvt"),
            &["mlt".to_string()]
        ));
    }

    #[test]
    fn an_extension_filter_rejects_a_path_without_an_extension() {
        assert!(!matches_extension_filter(
            Path::new("/tiles/README"),
            &["mlt".to_string()]
        ));
    }

    #[test]
    fn a_lone_info_row_renders_without_a_total_row() {
        insta::assert_snapshot!(render_table(
            &[info_row(mlt_info("a.mlt"))],
            LsFlags::default()
        ));
    }

    #[test]
    fn the_gzip_flag_adds_the_gzipped_and_gz_percent_columns() {
        insta::assert_snapshot!(render_table(&[info_row(mlt_info("a.mlt"))], GZIP));
    }

    #[test]
    fn the_algorithms_flag_adds_the_algorithms_column() {
        insta::assert_snapshot!(render_table(&[info_row(mlt_info("a.mlt"))], ALGORITHMS));
    }

    #[test]
    fn two_mlt_rows_are_summed_into_a_total_row() {
        insta::assert_snapshot!(render_table(
            &[info_row(mlt_info("a.mlt")), info_row(mlt_info("b.mlt"))],
            GZIP
        ));
    }

    #[test]
    fn a_total_row_over_files_without_decoded_or_gzip_sizes_shows_dashes() {
        insta::assert_snapshot!(render_table(
            &[info_row(mvt_info("a.mvt")), info_row(mvt_info("b.mvt"))],
            GZIP
        ));
    }

    #[test]
    fn a_mixed_total_row_falls_back_to_dashes_for_the_decoded_columns() {
        insta::assert_snapshot!(render_table(
            &[info_row(mlt_info("a.mlt")), info_row(mvt_info("b.mvt"))],
            GZIP
        ));
    }

    #[test]
    fn an_error_row_spans_the_remaining_columns() {
        insta::assert_snapshot!(render_table(
            &[
                info_row(mlt_info("a.mlt")),
                error_row("broken.mlt", Some(777)),
                error_row("missing.mlt", None),
            ],
            LsFlags::default()
        ));
    }

    #[test]
    fn validating_marks_every_mismatching_row_with_a_cross() {
        insta::assert_snapshot!(render_table(
            &[
                info_row(validated(mlt_info("a.mlt"), false)),
                info_row(validated(mlt_info("b.mlt"), false)),
            ],
            VALIDATE
        ));
    }

    #[test]
    fn validating_hides_a_matching_row_but_keeps_an_error_row() {
        insta::assert_snapshot!(render_table(
            &[
                info_row(validated(mlt_info("a.mlt"), true)),
                error_row("broken.mlt", Some(777)),
            ],
            VALIDATE
        ));
    }

    #[test]
    fn validating_an_unvalidated_row_shows_a_dash_in_the_json_column() {
        insta::assert_snapshot!(render_table(&[info_row(mlt_info("a.mlt"))], VALIDATE));
    }
}
