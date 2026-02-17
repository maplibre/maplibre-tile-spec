#![no_main]

use mlt_fuzz::LayerInput;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: LayerInput| {
    input.fuzz_roundtrip();
});
