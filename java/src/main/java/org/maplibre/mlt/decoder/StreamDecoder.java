package org.maplibre.mlt.decoder;

import java.io.IOException;
import me.lemire.integercompression.IntWrapper;
import org.maplibre.mlt.metadata.stream.*;

public class StreamDecoder {

  public static StreamMetadata decode(byte[] tile, IntWrapper offset) throws IOException {
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
