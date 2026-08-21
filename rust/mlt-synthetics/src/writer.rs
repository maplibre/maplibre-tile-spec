use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use mlt_core::encoder::WireVersion;
use mlt_core::geojson::FeatureCollection;
use mlt_core::{Decoder, MltError, Parser};

use crate::Args;
use crate::layer::Layer;

const WIRE_VERSIONS_CNT: usize = 2;

pub struct SynthWriter {
    ref_dirs: [PathBuf; WIRE_VERSIONS_CNT],
    out_dirs: [PathBuf; WIRE_VERSIONS_CNT],
    verbose: bool,
    notes: usize,
    pub failures: usize,
    /// all generated version files to prevent dup naming issues
    generated: HashSet<(WireVersion, String)>,
    /// Encoded bytes -> the fixture name that produced them to prevent dup content
    by_content: HashMap<Vec<u8>, String>,
}

pub type SynthResult<T> = Result<T, SynthErr>;

#[derive(Debug, thiserror::Error)]
pub enum SynthErr {
    #[error(transparent)]
    Mlt(#[from] MltError),
    #[error("cannot read reference MLT file: {0}")]
    ReadRefMlt(#[source] std::io::Error),
    #[error("MLT mismatch: reference file {} does not match generated content. Content saved to -rust dir.", .0.display())]
    MltMismatch(PathBuf),
    #[error("cannot read reference JSON file: {0}")]
    ReadRefJson(#[source] std::io::Error),
    #[error("decoded JSON differs from reference")]
    JsonMismatch,
    #[error("cannot parse reference as FeatureCollection: {0}")]
    UnparsableRef(serde_json::Error),
    #[error("cannot compare FeatureCollections: {0}")]
    CannotCompare(serde_json::Error),
    #[error("cannot serialize FeatureCollection: {0}")]
    SerializeJson(serde_json::Error),
    #[error("cannot write {0}: {1}")]
    WriteFile(PathBuf, #[source] std::io::Error),
    #[error(
        "{0:?} bytes are byte-identical to fixture `{1}`, so this fixture adds no coverage; \
         drop whichever builder call was meant to change the output (e.g. a no-op force_empty_stream)"
    )]
    DuplicateContent(WireVersion, String),
}

/// Compare `actual` against the JSON reference file at `ref_path`.
/// Returns `Ok(())` on match, or a typed `SynthError` on I/O error, parse failure, or mismatch.
pub fn check_json(actual: &FeatureCollection, ref_path: &Path) -> SynthResult<()> {
    let ref_json = fs::read_to_string(ref_path).map_err(SynthErr::ReadRefJson)?;
    let expected = FeatureCollection::from_str(&ref_json).map_err(SynthErr::UnparsableRef)?;
    if actual.equals(&expected).map_err(SynthErr::CannotCompare)? {
        Ok(())
    } else {
        Err(SynthErr::JsonMismatch)
    }
}

pub fn write_file(path: &Path, data: &[u8]) -> SynthResult<()> {
    Layer::open_new(path)
        .and_then(|mut f| f.write_all(data))
        .map_err(|source| SynthErr::WriteFile(path.to_path_buf(), source))
}

pub fn decode_to_json(bytes: &[u8]) -> FeatureCollection {
    let mut dec = Decoder::default();
    let decoded = dec
        .decode_all(Parser::default().parse_layers(bytes).unwrap())
        .unwrap();
    FeatureCollection::from_layers(decoded).unwrap()
}

impl SynthWriter {
    pub fn new(args: Args) -> Self {
        let root = args.synthetics.canonicalize().unwrap_or_else(|e| {
            panic!(
                "synthetics dir not found: {}\n{e}",
                args.synthetics.display()
            )
        });
        let ref_dirs = [root.join("0x01"), root.join("0x02")];
        ref_dirs.iter().for_each(|d| assert!(d.is_dir()));
        let out_dirs = [root.join("0x01-rust"), root.join("0x02-rust")];

        println!(
            "Verifying synthetics against {:?}",
            ref_dirs
                .iter()
                .map(|p| format!("{:?}", p.display()))
                .collect::<Vec<_>>()
        );
        println!(
            "Writing rust-only files to {:?}",
            out_dirs
                .iter()
                .map(|p| format!("{:?}", p.display()))
                .collect::<Vec<_>>()
        );
        for d in &out_dirs {
            fs::create_dir_all(&d).unwrap_or_else(|e| panic!("cannot create {}: {e}", d.display()));
        }

        Self {
            ref_dirs,
            out_dirs,
            verbose: args.verbose,
            failures: 0,
            notes: 0,
            generated: HashSet::new(),
            by_content: HashMap::new(),
        }
    }

    pub fn print_note(&mut self, msg: &str) {
        self.notes += 1;
        eprintln!("Note: {msg}");
    }

    /// Encode and write (or verify) `layer`, recording the outcome in this writer's statistics.
    pub fn write(&mut self, layer: Layer, name: impl AsRef<str>) {
        let name = name.as_ref();
        let res = self.write_int(layer, name);
        match res {
            Ok(()) => {}
            Err(e) => {
                eprintln!("FAIL {name}: {e}");
                self.failures += 1;
            }
        }
    }

    /// Encode `layer` and either verify (shared files) or write (rust-only files).
    ///
    /// Returns `Ok(true)` for a rust-only file, `Ok(false)` for a shared file,
    /// or `Err` on any failure.
    fn write_int(&mut self, layer: Layer, mut name: &str) -> SynthResult<()> {
        let mut is_rust_specific = false;
        if let Some(base) = name.strip_suffix("-rust") {
            is_rust_specific = true;
            name = base;
        }
        if name.contains("_fsst") {
            // FSST frequently generates binary-different but compatible data
            is_rust_specific = true;
        }
        let name_mlt = format!("{name}.mlt");
        let name_json = format!("{name}.json");
        let versions = if layer.wants_v2() {
            [WireVersion::V01, WireVersion::V02].as_slice()
        } else {
            [WireVersion::V01].as_slice()
        };
        for ((ref_dir, out_dir), &version) in self
            .ref_dirs
            .clone()
            .into_iter()
            .zip(self.out_dirs.clone())
            .zip(versions)
        {
            let rust_mlt = out_dir.join(&name_mlt);
            let rust_json = out_dir.join(&name_json);
            let ref_mlt = ref_dir.join(&name_mlt);
            let ref_json = ref_dir.join(&name_json);
            let ref_json_exists = ref_json.is_file();
            let bytes = layer.clone().encode_to_bytes(version)?;
            assert!(
                self.generated.insert((version, name.to_owned())),
                "expected to not generate the same name more than once, got {name} for {version:?}"
            );
            if let Some(prev) = self.by_content.insert(bytes.clone(), name.to_owned()) {
                return Err(SynthErr::DuplicateContent(version, prev));
            }
            let decoded = decode_to_json(&bytes);

            // The `-rust` suffix and `_fsst` marker say Java's encoder *may* differ, but that
            // is a per-version question: a fixture Java only lacks in v1 still has a correct
            // v2 reference. When this version's reference already matches byte-for-byte there
            // is nothing version-specific to record, so verify against it rather than write a
            // redundant `{ver}-rust/` copy (which `_assert-all-mlt-files-different` then flags
            // as a duplicate).
            let is_rust_specific = is_rust_specific
                && !(ref_json_exists && fs::read(&ref_mlt).is_ok_and(|r| r == bytes));

            if is_rust_specific || !ref_json_exists {
                // rust-only: write MLT to disk, compare decoded JSON to reference (if it exists).
                write_file(&rust_mlt, &bytes)?;
                if ref_json_exists {
                    check_json(&decoded, &ref_json)?;
                } else {
                    self.print_note(&format!(
                        "Synthetics doesn't have MLT matching 0x01-rust/{name_mlt}"
                    ));
                }
                let mut s =
                    serde_json::to_string_pretty(&decoded).map_err(SynthErr::SerializeJson)?;
                s.push('\n');
                write_file(&rust_json, s.as_bytes())?;
                if self.verbose {
                    println!("wrote  {name}");
                }
            } else {
                // shared: verify bytes and JSON against reference, nothing written to disk.
                fs::read(&ref_mlt)
                    .map_err(SynthErr::ReadRefMlt)
                    .and_then(|ref_bytes| {
                        if ref_bytes == bytes {
                            Ok(())
                        } else {
                            write_file(&rust_mlt, &bytes)?;
                            Err(SynthErr::MltMismatch(ref_mlt))
                        }
                    })?;
                check_json(&decoded, &ref_json)?;
                if self.verbose {
                    println!("ok  {name}");
                }
            };
        }

        Ok(())
    }

    /// Warn about `.mlt` files in the reference dir that Rust never generated.
    /// Prints a summary that includes the total failure count.
    pub fn report_ungenerated(&mut self) {
        let versions: [WireVersion; WIRE_VERSIONS_CNT] = [WireVersion::V01, WireVersion::V02];
        for (d, v) in self.ref_dirs.clone().iter().zip(versions) {
            let mut ref_mlts: Vec<String> = fs::read_dir(d)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", d.display()))
                .flatten()
                .filter_map(|e| {
                    let p = e.path();
                    (p.extension()? == "mlt")
                        .then(|| p.file_stem().unwrap().to_string_lossy().into_owned())
                })
                .collect();
            ref_mlts.sort();
            for name in &ref_mlts {
                if !self.generated.contains(&(v, name.clone())) {
                    self.print_note(&format!(
                        "Rust synthetics did not generate a test matching Java's 0x01/{name}.mlt"
                    ));
                }
            }
        }

        println!(
            "Generated {} | Notes: {} | Failures: {}",
            self.generated.len(),
            self.notes,
            self.failures,
        );
    }
}
