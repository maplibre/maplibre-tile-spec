The metadata sections of a COVT tile describes the structure of a layer.
Since new layers should be easily added to existing tiles, there is no per tile header.
The actual data per layer are preceded by a metadata section which contains the data like the used
encodings or the number of features.
Since the metadata can be a dominant portion of the overall size in particular for small tiles
 it is important to keep
the metadata section as minimal as possible.
Since the metadata can make up a dominant part of the total size, especially with small tiles (zoom > 8 where size per tile often <5 kb on some optimized vector tiles schemes),
it is important to keep the metadata section as small as possible.
A large part of the metadata are the strings for the layer and column names.
Therefore COVT has also an option where the names are replaced by ids (u32) and stored in a central separate file (TileJSON).

The metadata section for a layer of a COVT tile has the following structure:

![](./assets/metadata.png)
