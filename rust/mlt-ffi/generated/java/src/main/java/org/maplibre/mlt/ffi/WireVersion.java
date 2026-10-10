package org.maplibre.mlt.ffi;

import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/** The wire format an encoded layer uses. */
public enum WireVersion {
  /** Tag {@code 0x01}, the stable v1 format. */
  V01,
  /** Tag {@code 0x02}, the experimental v2 format. */
  V02;

  int nativeValue() {
    return this == V01 ? mlt_ffi_h.MltWireVersion_V01() : mlt_ffi_h.MltWireVersion_V02();
  }
}
