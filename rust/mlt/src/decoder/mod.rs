pub mod boolean;
mod decode;
mod helpers;
pub mod integer;
pub mod integer_stream;
pub mod tracked_bytes;
pub mod varint;

#[cfg(test)]
#[allow(unused_imports)]
#[allow(clippy::needless_for_each)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering::Relaxed;

    use insta::with_settings;
    use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};

    use crate::decoder::integer::decode_int_stream;
    use crate::decoder::tracked_bytes::TrackedBytes;
    use crate::metadata::stream::StreamMetadata;

    /// Returns a list of (string name, path stem) for all files in the expected directory.
    fn get_bin_fixtures() -> Vec<(String, PathBuf)> {
        let root = Path::new("../../test/expected");
        walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(|entry| {
                let path = entry.ok()?;
                let path = path.path().strip_prefix(root).ok()?;
                // Search for .meta.bin files, and return
                let mut filename = path
                    .file_name()?
                    .to_str()?
                    .strip_suffix(".meta.bin")?
                    .to_owned();
                let rel_stem_name = path.parent()?.join(&filename).to_str()?.to_owned();
                filename += ".bin";
                let bin_path = root.join(path.parent()?.join(filename)).clone();
                Some((rel_stem_name, bin_path))
            })
            .collect::<Vec<_>>()
    }

    #[ignore = "not updated for tag0x01 yet"]
    #[test]
    fn test_parse_meta_fixtures() {
        let count = AtomicUsize::new(0);
        get_bin_fixtures()
            // .into_par_iter()
            .into_iter()
            .for_each(|(name, path)| {
                let mut bytes: TrackedBytes = fs::read(path.with_extension("meta.bin"))
                    .expect(&name)
                    .into();
                let meta = StreamMetadata::decode(&mut bytes).expect(&name);
                assert!(bytes.is_empty(), "case {name}, remaining {bytes:?}");

                if cfg!(feature = "test-snapshots") {
                    with_settings!(
                        { snapshot_suffix => &name,
                          snapshot_path => "../../snapshots",
                          omit_expression => true,
                          prepend_module_to_snapshot => false },
                        { insta::assert_debug_snapshot!(meta) }
                    );
                } else {
                    eprintln!("{name} => {meta:?}");
                }

                count.fetch_add(1, Relaxed);
            });

        let count = count.load(Relaxed);
        eprintln!("Parsed {count} meta fixtures");
        assert!(count > 0);
    }

    #[test]
    #[ignore = "not all parsing has been implemented yet - some boolean test fixtures have incorrect JSON output due to Java encoder bug (see issue #569)"]
    fn test_decode_fixtures() {
        for (name, path) in &get_bin_fixtures() {
            let meta = fs::read(path.with_extension("meta.bin")).expect(name);
            let meta = StreamMetadata::decode(&mut meta.into()).expect(name);

            let data = fs::read(path.with_extension("bin")).expect(name);
            eprintln!("{name} => data {data:?}");
            assert!(!data.is_empty());

            // if meta.logical.technique1 == Morton
            let result = decode_int_stream(&mut data.into(), &meta, false).expect(name);

            // let result: Vec<_> =
            //     decode_byte_rle(&mut data.into(), (meta.num_values as usize).div_ceil(8))
            //         .into_iter()
            //         .map(|b| b != 0)
            //         .collect();
            // eprintln!("{name} => result {result:?}");

            let expected = fs::read_to_string(path.with_extension("json")).expect(name);
            // Note: Some boolean test fixtures may have incorrect JSON output due to Java encoder bug
            // where JSON is truncated at the last true bit (issue #569). The binary data is correct.
            assert_eq!(
                serde_json::to_string(&result).expect(name),
                expected,
                "case {name}"
            );

            // with_settings!(
            //     { snapshot_suffix => name,
            //       snapshot_path => "../../snapshots",
            //       omit_expression => true,
            //       prepend_module_to_snapshot => false },
            //     { insta::assert_debug_snapshot!(result) }
            // );
        }
    }
}
