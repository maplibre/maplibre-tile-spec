package org.maplibre.mlt.ffi;

import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/** The wire format an encoded layer uses. */
public enum MltWireVersion {
  /** Tag {@code 0x01}, the stable v1 format. */
  V1,
  /** Tag {@code 0x02}, the experimental v2 format. */
  V2;

  int nativeValue() {
    return switch (this) {
      case V1 -> mlt_ffi_h.MltWireVersion_V01();
      case V2 -> mlt_ffi_h.MltWireVersion_V02();
    };
  }
}
