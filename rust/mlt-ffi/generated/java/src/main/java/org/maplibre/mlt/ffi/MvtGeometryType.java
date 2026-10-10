package org.maplibre.mlt.ffi;

import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/** The geometry type of an MVT feature, which also covers its multi variant. */
public enum MvtGeometryType {
  /** A point or multi-point. */
  POINT,
  /** A line string or multi-line string. */
  LINE_STRING,
  /** A polygon or multi-polygon. */
  POLYGON;

  int nativeValue() {
    return switch (this) {
      case POINT -> mlt_ffi_h.MltMvtGeometryType_Point();
      case LINE_STRING -> mlt_ffi_h.MltMvtGeometryType_LineString();
      case POLYGON -> mlt_ffi_h.MltMvtGeometryType_Polygon();
    };
  }
}
