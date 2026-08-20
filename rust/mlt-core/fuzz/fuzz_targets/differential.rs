#![no_main]

use libfuzzer_sys::fuzz_target;
use mlt_fuzz::DifferentialInput;

fuzz_target!(|input: DifferentialInput| {
    input.fuzz();
});
