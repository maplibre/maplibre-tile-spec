package org.maplibre.mlt.ffi;

import org.maplibre.mlt.ffi.raw.mlt_ffi_h;

/** The geometry type of one feature. */
public enum GeometryType {
  POINT,
  LINE_STRING,
  POLYGON,
  MULTI_POINT,
  MULTI_LINE_STRING,
  MULTI_POLYGON;

  int nativeValue() {
    return switch (this) {
      case POINT -> mlt_ffi_h.MltGeometryType_Point();
      case LINE_STRING -> mlt_ffi_h.MltGeometryType_LineString();
      case POLYGON -> mlt_ffi_h.MltGeometryType_Polygon();
      case MULTI_POINT -> mlt_ffi_h.MltGeometryType_MultiPoint();
      case MULTI_LINE_STRING -> mlt_ffi_h.MltGeometryType_MultiLineString();
      case MULTI_POLYGON -> mlt_ffi_h.MltGeometryType_MultiPolygon();
    };
  }
}
