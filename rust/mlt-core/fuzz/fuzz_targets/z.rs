#![no_main]

use libfuzzer_sys::fuzz_target;
use mlt_fuzz::ZInput;

fuzz_target!(|input: ZInput| {
    input.fuzz();
});
