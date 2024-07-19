package com.mlt.converter.encodings.fsst;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.io.IOException;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

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
