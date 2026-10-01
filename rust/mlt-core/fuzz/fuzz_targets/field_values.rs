#![no_main]

use libfuzzer_sys::fuzz_target;
use mlt_fuzz::FieldValuesInput;

fuzz_target!(|input: FieldValuesInput| {
    input.fuzz();
});
