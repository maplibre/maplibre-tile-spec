use std::fmt::{Debug, Display, Formatter};

use hex::ToHex as _;

/// Wrapper type for optional slices to provide a custom Debug implementation
pub struct OptSeq<'a, T>(pub Option<&'a [T]>);

impl<T: Display + Debug> Debug for OptSeq<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write_seq(f, self.0, ToString::to_string)
    }
}

pub struct OptSeqOpt<'a, T>(pub Option<&'a [Option<T>]>);

impl<T: Display + Debug> Debug for OptSeqOpt<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write_seq(f, self.0, |opt| match opt {
            Some(val) => val.to_string(),
            None => "None".to_string(),
        })
    }
}

fn write_seq<T>(
    f: &mut Formatter,
    value: Option<&[T]>,
    to_str: fn(&T) -> String,
) -> std::fmt::Result {
    if let Some(v) = value {
        if f.alternate() {
            let items = v.iter().map(to_str).collect::<Vec<_>>().join(",");
            write!(f, "[{items}; {}]", v.len())
        } else {
            let items = v.iter().take(8).map(to_str).collect::<Vec<_>>().join(",");
            write!(f, "[{items}")?;
            if v.len() > 8 {
                write!(f, ", ...; {}]", v.len())
            } else {
                write!(f, "]")
            }
        }
    } else {
        write!(f, "None")
    }
}

pub(crate) fn fmt_byte_array(data: &[u8], f: &mut Formatter<'_>) -> std::fmt::Result {
    if f.alternate() {
        let vals = data.encode_hex_upper::<String>();
        write!(f, "[0x{vals}; {}]", data.len())
    } else {
        let vals = (&data[..8.min(data.len())]).encode_hex_upper::<String>();
        write!(
            f,
            "[0x{vals}{}; {}]",
            if data.len() <= 8 { "" } else { "..." },
            data.len()
        )
    }
}
