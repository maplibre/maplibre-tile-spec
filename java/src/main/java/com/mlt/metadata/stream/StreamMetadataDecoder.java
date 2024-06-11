package com.mlt.metadata.stream;

import me.lemire.integercompression.IntWrapper;

public class StreamMetadataDecoder {

  public static StreamMetadata decode(byte[] tile, IntWrapper offset) {
    var streamMetadata = StreamMetadata.decode(tile, offset);
    /* Currently morton can't be combined with RLE only with delta */
    if (streamMetadata.logicalLevelTechnique1().equals(LogicalLevelTechnique.MORTON)) {
      return MortonEncodedStreamMetadata.decodePartial(streamMetadata, tile, offset);
    }
    /* Boolean rle doesn't need additional information */
    else if ((LogicalLevelTechnique.RLE.equals(streamMetadata.logicalLevelTechnique1())
            || LogicalLevelTechnique.RLE.equals(streamMetadata.logicalLevelTechnique2()))
        && !PhysicalLevelTechnique.NONE.equals(streamMetadata.physicalLevelTechnique())) {
      return RleEncodedStreamMetadata.decodePartial(streamMetadata, tile, offset);
    }

    return streamMetadata;
  }
}
