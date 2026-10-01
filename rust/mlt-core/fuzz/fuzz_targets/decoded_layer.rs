#![no_main]

use libfuzzer_sys::fuzz_target;
use mlt_fuzz::DecodedLayerInput;

fuzz_target!(|input: DecodedLayerInput| {
    input.fuzz_roundtrip();
});
