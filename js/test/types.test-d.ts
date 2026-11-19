import {
  MltFeature,
  MltFeature3D,
  MltGeometryType,
  MltLayer,
  MltLayer3D,
  MltPoint,
  columnValue,
  decodeTile,
  decodeTile3D,
  decodeTileColumns,
  featureGeometry,
  geometryStarts,
  isMeshes,
  isTrianglesOnly,
  mValueSlice,
  tileGeoJson,
  toElevation,
  type MltColumnLayer,
  type MltGeoJsonFeatureCollection,
} from "@maplibre/mlt-js";
import {
  MltGeometryType as ColumnsGeomType,
  columnValue as columnsVal,
  decodeTileColumns as columnsDecode,
  mValueSlice as columnsMSlice,
  toElevation as columnsElev,
} from "@maplibre/mlt-js/columns";

const bytes = new Uint8Array(0);
const colTile = decodeTileColumns(bytes, { layers: ["buildings"], maxBytes: 64 * 1024 * 1024 });
const colTile2 = columnsDecode(bytes, { maxBytes: Infinity });
void colTile2;

for (const layer of colTile.layers) {
  const _checkLayer: MltColumnLayer = layer;
  void _checkLayer;
  if (layer.geometry.dimension === 3) {
    const m: number = toElevation(
      layer.geometry.vertices[2] ?? 0,
      layer.geometry.zStep,
      layer.geometry.zDatum,
    );
    const m2: number = columnsElev(0, layer.geometry.zStep);
    void m;
    void m2;
  }
  for (const prop of layer.properties) {
    const v = columnValue(prop, 0);
    const v2 = columnsVal(prop, 0);
    void v;
    void v2;
    if (prop.type === "f32") {
      const arr: Float32Array = prop.values;
      void arr;
    }
  }
  if (layer.mValues) {
    for (const mv of layer.mValues) {
      const slice = mValueSlice(layer, mv, 0);
      const slice2 = columnsMSlice(layer, mv, 0);
      void slice;
      void slice2;
    }
  }
  if (!isTrianglesOnly(layer.geometry)) {
    const starts = geometryStarts(layer.geometry);
    void starts.featureGeometries;
    const fg = featureGeometry(layer, 0);
    if (fg.kind === "mesh") {
      const tris: Uint32Array = fg.triangles;
      void tris;
    }
  }
  void isMeshes(layer.geometry);
}

const meshCode: number = MltGeometryType.Mesh;
const meshName: string = ColumnsGeomType[6];
void meshCode;
void meshName;

const vt = decodeTile(bytes, { maxBytes: 1024 });
const vt3d = decodeTile3D(bytes, { maxBytes: 1024 });
const fc: MltGeoJsonFeatureCollection = tileGeoJson(bytes, { maxBytes: 1024 });
const pt = new MltPoint(1, 2).clone();
void vt;
void vt3d;
void fc;
void pt;
void MltFeature;
void MltFeature3D;
void MltLayer;
void MltLayer3D;
