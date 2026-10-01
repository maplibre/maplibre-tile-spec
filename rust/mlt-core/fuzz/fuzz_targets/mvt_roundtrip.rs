#![no_main]

use libfuzzer_sys::fuzz_target;
use mlt_fuzz::MvtRoundtripInput;

fuzz_target!(|input: MvtRoundtripInput| {
    input.fuzz_roundtrip();
});
