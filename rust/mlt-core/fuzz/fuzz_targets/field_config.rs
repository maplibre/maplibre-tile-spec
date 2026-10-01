#![no_main]

use libfuzzer_sys::fuzz_target;
use mlt_fuzz::FieldConfigInput;

fuzz_target!(|input: FieldConfigInput| {
    input.fuzz();
});
