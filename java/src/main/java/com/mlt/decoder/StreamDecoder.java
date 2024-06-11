package com.mlt.decoder;

import com.mlt.metadata.stream.*;
import me.lemire.integercompression.IntWrapper;

public class StreamDecoder {

  public static StreamMetadata decode(byte[] tile, IntWrapper offset) {
    var streamMetadata = StreamMetadata.decode(tile, offset);

    if (LogicalLevelTechnique.RLE.equals(streamMetadata.logicalLevelTechnique1())
        || LogicalLevelTechnique.RLE.equals(streamMetadata.logicalLevelTechnique2())) {
      return RleEncodedStreamMetadata.decodePartial(streamMetadata, tile, offset);
    }

    if (LogicalLevelTechnique.MORTON.equals(streamMetadata.logicalLevelTechnique1())) {
      return MortonEncodedStreamMetadata.decodePartial(streamMetadata, tile, offset);
    }

    return streamMetadata;
  }
}
