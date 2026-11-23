export { default as decodeTile } from "./mltDecoder";
export { encodeTile } from "./encoding/mltEncoder";
export { default as FeatureTable, type FeatureTableV2Meta } from "./vector/featureTable";
export { GeometryVector } from "./vector/geometry/geometryVector";
export { GpuVector } from "./vector/geometry/gpuVector";
export { default as GeometryScaling } from "./decoding/geometryScaling";
export { GEOMETRY_TYPE } from "./vector/geometry/geometryType";
export type { TileSetMetadata } from "./metadata/tileset/tilesetMetadata";
export type { Geometry } from "./vector/geometry/geometryVector";
export type { Feature } from "./vector/featureTable";
export type { MapEncodingOptions } from "./encoding/mapPropertyEncoder";
export type {
    EncodeOptions,
    Feature as FeatureInput,
    FeatureGeometry,
    Layer,
    Position,
    PropertyType,
    PropertyValue,
} from "./encoding/mltEncoder";

export {
    columnValue,
    type DecodeTileColumnsOptions,
    decodeTileColumns,
    type HeightMode,
    type MltColumnLayer,
    type MltColumnScalar,
    type MltColumnTile,
    type MltGeometryColumns,
    type MltGeometryLayout,
    MltGeometryType,
    type MltHeightMode,
    type MltIdColumn,
    type MltMValueColumn,
    type MltMValueScope,
    type MltMValueSlice,
    type MltNestedColumn,
    type MltPropertyColumn,
    type MltScalarMValueColumn,
    type MltScalarPropertyColumn,
    type MltStringMValueColumn,
    type MltStringPropertyColumn,
    mValueSlice,
    toElevation,
} from "./columns";

export {
    featureGeometry,
    geometryStart,
    geometryStarts,
    isMeshes,
    isTrianglesOnly,
    lineStart,
    type MltFeatureGeometry,
    type MltGeometryStarts,
    type MltLineGeometry,
    type MltMeshGeometry,
    type MltPointGeometry,
    type MltPolygonGeometry,
    type MltPolygonRings,
    type MltVertexRun,
    meshTriangles,
} from "./featureGeometry";

export {
    type DecodeTileOptions,
    decodeTile3D,
    decodeVectorTile,
    MltFeature,
    MltFeature3D,
    MltLayer,
    MltLayer3D,
    MltPoint,
    type MltTile,
    type MltTile3D,
    type Position3D,
} from "./vectorTile";

export { type MltGeoJsonFeature, tileGeoJson } from "./geojson";
