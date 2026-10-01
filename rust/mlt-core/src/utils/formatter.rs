use std::fmt::{self, Debug, Display, Formatter};

use hex::ToHex as _;

fn format_byte_array(data: &[u8]) -> String {
    let vals = data.encode_hex_upper::<String>();
    format!("[0x{vals}; {}]", data.len())
}

/// derive-debug formatter for `&[u8]` fields.
pub fn bytes_dbg(data: &&[u8]) -> String {
    format_byte_array(data)
}

fn format_opt_seq<T: Display>(v: Option<&[T]>) -> String {
    match v {
        None => "None".to_string(),
        Some(v) => {
            let items = v
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{items}; {}]", v.len())
        }
    }
}

/// derive-debug formatter for `Vec<T>` fields.
pub fn vec_seq<T: Display>(v: &[T]) -> String {
    format_opt_seq(Some(v))
}

/// derive-debug formatter for `Option<Vec<T>>` fields.
#[allow(clippy::ref_option, reason = "called by Dbg codegen")]
pub fn opt_vec_seq<T: Display>(v: &Option<Vec<T>>) -> String {
    format_opt_seq(v.as_deref())
}

/// Wraps any `Debug` value and formats it in compact (non-alternate) mode.
/// Used to prevent inner types from expanding to multiple lines in `{:#?}` output.
pub struct CompactDbg<T>(T);

impl<T: Debug> Display for CompactDbg<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// derive-debug formatter that forces compact `{:?}` output regardless of alternate mode.
pub fn compact_dbg<T: Debug>(t: &T) -> CompactDbg<&T> {
    CompactDbg(t)
}
