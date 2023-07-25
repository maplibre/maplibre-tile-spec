Goal
- Understand how a MVT tile is accessed and processed 

Questions
- when and where in the code are the geometry and properties used?

Workflow for decoding and processing a MVT tile in MapLibre-GL:
- Parse and process MVT
  - worker -> load vector tile
  - worker_source -> getArrayBuffer, decode pbf, parse vector tiles
  - Calls VectorTileWorkerSource#loadVectorTile 
    - which uses ajax#getArrayBuffer() to fetch raw bytes 
    - pbf library to decode the protobuf
    - @mapbox/vector-tile#VectorTile to parse the vector tile
    - The result goes into a new WorkerTile instance
  - Calls WorkerTile#parse() and caches the result in the worker by tile ID:
    - For each vector tile source layer, for each style layer that depends on the source layer that is currently visible ("layer family"):
       - Calculate layout properties (recalculateLayers)
       - Call style.createBucket, which delegates to a bucket type in src/data/bucket/*, which are subclasses of src/data/bucket
       - Call Bucket#populate() with the features from this source layer in the vector tile. This precomputes all the data that the main thread needs to load into the GPU to render each frame (ie. buffers containing vertices of all the triangles that compose the shape)
- Bucket
  - worker_tile.populate -> compute triangles for each feature in "layer family" 
  - for each "layer family" create bucket in worker_tile  
  - compute triangles for each feature in a "layer family" 
  - place characters
  - compute collision boxes
  - returned to source -> bucket, featureIndex, collision boxes 

- Bucket -> Transforming that data into render-ready data that can be used by WebGL shaders to draw the map
- FeatureIndex -> Indexing feature geometries into a FeatureIndex, used for spatial queries (e.g. queryRenderedFeatures)