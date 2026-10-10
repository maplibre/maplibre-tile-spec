package org.maplibre.mlt.ffi;

import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/** The geometry type of an MVT feature, which also covers its multi variant. */
public enum MvtGeometryType {
  POINT,
  LINE_STRING,
  POLYGON;

  int nativeValue() {
    return switch (this) {
      case POINT -> mlt_ffi_h.MltMvtGeometryType_Point();
      case LINE_STRING -> mlt_ffi_h.MltMvtGeometryType_LineString();
      case POLYGON -> mlt_ffi_h.MltMvtGeometryType_Polygon();
    };
  }
}
