# Changelog

All notable changes to the TypeScript MapLibre Tile decoder.

## [2025-11-14]

### Added
- Implemented `SequenceSelectionVector` for memory-efficient sequential selections
- Added winding order validation for polygons (MVT spec compliance)
- New `calculateSignedArea()` function using shoelace formula
- Inline documentation for selection vectors and geometry processing

### Changed
- Updated `filter.ts` to use `SequenceSelectionVector` for no-filter cases
- Updated `selectionVectorUtils.ts` to use `SequenceSelectionVector` by default
- Updated `constGeometryVector.ts` and `constGpuVector.ts` to use `SequenceSelectionVector`
- Modified `createPolygon()` and `createMultiPolygon()` to validate and correct winding order

### Removed
- Resolved TODOs in `filter.ts` (3 removed)
- Resolved TODO in `geometryVectorConverter.ts` (1 removed)

## [2025-11-12]

### Added
- Re-added flat vector implementations: Integer, Long, Float, Double, Boolean, String
- Added constant vectors: Integer, Long
- Added sequence vectors: Integer, Long (RLE-encoded)
- Added dictionary vectors: String, FSST-compressed String
- Implemented selection vectors: Flat (array-based), utilities
- Added complete filtering system (`filter.ts`) supporting MapLibre expressions
- Comprehensive test coverage

### Changed
- Enhanced base `vector.ts` with filtering, comparison, and match methods
- Updated geometry vectors (Const/Flat GeometryVector and GpuVector) with filtering support
- Exported new vector types in `index.ts`
