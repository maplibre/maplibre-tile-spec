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
    use insta::with_settings;
    use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering::Relaxed;

    use crate::decoder::helpers::decode_boolean_rle;
    use crate::decoder::integer::decode_int_stream;
    use crate::decoder::tracked_bytes::TrackedBytes;
    use crate::metadata::stream::StreamMetadata;
    use crate::metadata::stream_encoding::PhysicalStreamType;

    /// Returns a list of (string name, path stem) for all files in the fixtures directory.
    fn get_bin_fixtures() -> Vec<(String, PathBuf)> {
        let root = Path::new("../../test/fixtures");
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
    #[ignore = "not all parsing has been implemented yet"]
    fn test_decode_fixtures() {
        for (name, path) in &get_bin_fixtures() {
            let meta = fs::read(path.with_extension("meta.bin")).expect(name);
            let meta = StreamMetadata::decode(&mut meta.into()).expect(name);

            let data = fs::read(path.with_extension("bin")).expect(name);
            eprintln!("{name} => data {data:?}");
            assert!(!data.is_empty());

            // TODO(Weixing): implement the rest of the decoding
            eprintln!("----------------------------------------------");
            eprintln!("fixture name = {name}");
            eprintln!("fixture path = {}", path.display());
            eprintln!("decoded meta = {:#?}", meta);
            eprintln!("raw data stream (hex): {:02x?}", data);

            // read the expected json file
            let expected = fs::read_to_string(path.with_extension("json")).expect(name);
            eprintln!("expected: {}", expected);

            let result = match meta.physical.r#type {
                PhysicalStreamType::Present => {
                    // Use boolean decoder for PRESENT streams
                    use crate::decoder::helpers::decode_boolean_rle;
                    let bytes = decode_boolean_rle(&mut data.into(), meta.num_values as usize);
                    // Convert bytes to booleans
                    let mut booleans = Vec::new();
                    for byte in bytes {
                        for i in 0..8 {
                            booleans.push((byte & (1 << i)) != 0);
                        }
                    }
                    booleans.truncate(meta.num_values as usize);
                    serde_json::to_string(&booleans).expect(name)
                }
                PhysicalStreamType::Length | PhysicalStreamType::Offset => {
                    // Use integer decoder for LENGTH and OFFSET streams
                    let result = decode_int_stream(&mut data.into(), &meta, false).expect(name);
                    serde_json::to_string(&result).expect(name)
                }
                PhysicalStreamType::Data => {
                    // TODO: Implement data stream decoder
                    return;
                }
            };

            eprintln!("result:   {}", result);
            assert_eq!(result, expected, "case {name}");
        }
    }
}
