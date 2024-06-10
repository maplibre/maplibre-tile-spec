package com.mlt.converter.encodings;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.mlt.converter.encodings.fsst.FsstEncoder;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import org.junit.jupiter.api.Test;

public class FsstEncoderTest {

  @Test
  public void decode_simpleString_ValidEncodedAndDecoded() throws IOException {
    var expectedData = "AAAAAAABBBAAACCdddddEEEEEEfffEEEEAAAAAddddCC";

    var symbolTable = FsstEncoder.encode(expectedData.getBytes(StandardCharsets.UTF_8));
    var actualData =
        FsstEncoder.decode(
            symbolTable.symbols(), symbolTable.symbolLengths(), symbolTable.compressedData());

    assertEquals(expectedData, new String(actualData, StandardCharsets.UTF_8));
  }
}
