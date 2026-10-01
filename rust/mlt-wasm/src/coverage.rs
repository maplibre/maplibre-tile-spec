//! Export of the LLVM coverage counters recorded inside the wasm sandbox.

use wasm_bindgen::prelude::*;

/// Take the coverage profile recorded so far, as `.profraw` bytes.
#[wasm_bindgen(js_name = coverageDump)]
#[expect(unsafe_code, reason = "the only way to read the counters out of wasm")]
pub fn coverage_dump() -> Vec<u8> {
    let mut profraw = Vec::new();
    // SAFETY: wasm runs single-threaded here, so no other thread captures concurrently
    unsafe { minicov::capture_coverage(&mut profraw) }.expect("failed to capture coverage");
    profraw
}
