use std::collections::HashSet;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use mlt_core::geojson::FeatureCollection;
use mlt_core::{Decoder, MltError, Parser};

use crate::Args;
use crate::layer::{Layer, SynthWriter};

pub type SynthResult<T> = Result<T, SynthErr>;

#[derive(Debug, thiserror::Error)]
pub enum SynthErr {
    #[error("MLT error: {0}")]
    Mlt(#[source] MltError),
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

impl Layer {
    /// Encode and then either verify against the reference dir (non-rust files) or write to the
    /// output dir (`-rust`-suffixed files). Delegates to [`SynthWriter::write`].
    pub fn write(self, w: &mut SynthWriter, name: impl AsRef<str>) {
        w.write(self, name);
    }
}

impl SynthWriter {
    pub fn new(mut args: Args) -> Self {
        let canonical_synth = args.synthetics.canonicalize();
        let canonical_synth = canonical_synth.unwrap_or_else(|e| {
            panic!(
                "reference synthetics dir not found: {}\n{e}",
                args.synthetics.display()
            )
        });
        args.synthetics = canonical_synth;

        println!("Verifying synthetics against {}", args.synthetics.display());
        println!(
            "Writing rust-only files to {}",
            args.synthetics_rust.display()
        );

        fs::create_dir_all(&args.synthetics_rust)
            .unwrap_or_else(|e| panic!("cannot create {}: {e}", args.synthetics_rust.display()));

        Self {
            ref_dir: args.synthetics,
            out_dir: args.synthetics_rust,
            verbose: args.verbose,
            failures: 0,
            generated: HashSet::new(),
            rust_written: 0,
        }
    }

    /// Encode and write (or verify) `layer`, recording the outcome in this writer's statistics.
    pub fn write(&mut self, layer: Layer, name: impl AsRef<str>) {
        let name = name.as_ref();
        let res = self.write_int(layer, name);
        match res {
            Ok(is_rust) => {
                let typ = if is_rust {
                    self.rust_written += 1;
                    "wrote"
                } else {
                    assert!(
                        self.generated.insert(name.to_string()),
                        "duplicate generated name: {name}"
                    );
                    "ok"
                };
                if self.verbose {
                    println!("{typ:5}  {name}");
                }
            }
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
    fn write_int(&self, layer: Layer, name: &str) -> SynthResult<bool> {
        let name_mlt = format!("{name}.mlt");
        let out_mlt = self.out_dir.join(&name_mlt);
        let name_json = format!("{name}.json");
        let bytes = layer.encode_to_bytes()?;
        let decoded = decode_to_json(&bytes);

        if let Some(base) = name.strip_suffix("-rust") {
            // rust-only: write MLT to disk, compare decoded JSON to reference (if it exists).
            write_file(&out_mlt, &bytes)?;
            let name_base = format!("{base}.json");
            let ref_json_path = self.ref_dir.join(&name_base);
            if ref_json_path.is_file() {
                check_json(&decoded, &ref_json_path)?;
            } else {
                eprintln!("Note: {name_mlt} has no corresponding ../0x01/{name_base}");
            }
            let mut s = serde_json::to_string_pretty(&decoded).map_err(SynthErr::SerializeJson)?;
            s.push('\n');
            write_file(&self.out_dir.join(name_json), s.as_bytes())?;
            Ok(true)
        } else {
            // shared: verify bytes and JSON against reference, nothing written to disk.
            let path = self.ref_dir.join(&name_mlt);
            fs::read(&path)
                .map_err(SynthErr::ReadRefMlt)
                .and_then(|ref_bytes| {
                    if ref_bytes == bytes {
                        Ok(())
                    } else {
                        write_file(&out_mlt, &bytes)?;
                        Err(SynthErr::MltMismatch(path))
                    }
                })?;
            check_json(&decoded, &self.ref_dir.join(name_json))?;
            Ok(false)
        }
    }

    /// Warn about `.mlt` files in the reference dir that Rust never generated.
    /// Prints a summary and returns the total failure count.
    pub fn report_ungenerated(&self) {
        let mut ref_mlts: Vec<String> = fs::read_dir(&self.ref_dir)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", self.ref_dir.display()))
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                (p.extension()? == "mlt")
                    .then(|| p.file_stem().unwrap().to_string_lossy().into_owned())
            })
            .collect();
        ref_mlts.sort();

        let mut notes = 0;
        for name in &ref_mlts {
            if !self.generated.contains(name) {
                eprintln!("Note: Rust synthetics did not generate {name}.mlt");
                notes += 1;
            }
        }

        println!(
            "Verified: {} | Rust-only: {} | Notes: {notes} | Failures: {}",
            self.generated.len(),
            self.rust_written,
            self.failures,
        );
    }
}
