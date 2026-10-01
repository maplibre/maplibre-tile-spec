#![no_main]

use libfuzzer_sys::fuzz_target;
use mlt_fuzz::LayerInput;

fuzz_target!(|input: LayerInput| {
    input.fuzz_roundtrip();
});
