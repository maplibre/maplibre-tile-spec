#![no_main]

use libfuzzer_sys::fuzz_target;
use mlt_fuzz::WireVersionInput;

fuzz_target!(|input: WireVersionInput| {
    input.fuzz();
});
