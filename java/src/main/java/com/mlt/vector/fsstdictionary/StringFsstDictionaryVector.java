package com.mlt.vector.fsstdictionary;

import com.mlt.converter.encodings.fsst.FsstEncoder;
import com.mlt.vector.BitVector;
import com.mlt.vector.VariableSizeVector;
import java.nio.ByteBuffer;
import java.nio.IntBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

public class StringFsstDictionaryVector extends VariableSizeVector<String> {
  // TODO: extend from StringVector

  private IntBuffer dictionaryOffsetBuffer;
  private IntBuffer symbolTableOffsetBuffer;
  private IntBuffer indexBuffer;
  private IntBuffer symbolLengthBuffer;
  private ByteBuffer symbolTableBuffer;
  private List<String> decodedValues;

  public StringFsstDictionaryVector(
      String name,
      IntBuffer indexBuffer,
      IntBuffer lengthBuffer,
      ByteBuffer dictionaryBuffer,
      IntBuffer symbolLengthBuffer,
      ByteBuffer symbolTableBuffer) {
    super(name, lengthBuffer, dictionaryBuffer);
    setBuffer(indexBuffer, symbolLengthBuffer, symbolTableBuffer);
  }

  public StringFsstDictionaryVector(
      String name,
      BitVector nullabilityBuffer,
      IntBuffer indexBuffer,
      IntBuffer lengthBuffer,
      ByteBuffer dictionaryBuffer,
      IntBuffer symbolLengthBuffer,
      ByteBuffer symbolTableBuffer) {
    super(name, nullabilityBuffer, lengthBuffer, dictionaryBuffer);
    setBuffer(indexBuffer, symbolLengthBuffer, symbolTableBuffer);
  }

  public static StringFsstDictionaryVector createFromOffsetBuffer(
      String name,
      BitVector nullabilityBuffer,
      IntBuffer indexBuffer,
      IntBuffer offsetBuffer,
      ByteBuffer dictionaryBuffer,
      IntBuffer symbolOffsetBuffer,
      ByteBuffer symbolTableBuffer) {
    var vector =
        new StringFsstDictionaryVector(
            name, nullabilityBuffer, indexBuffer, null, dictionaryBuffer, null, symbolTableBuffer);
    vector.dictionaryOffsetBuffer = offsetBuffer;
    vector.symbolTableOffsetBuffer = symbolOffsetBuffer;
    return vector;
  }

  private void setBuffer(
      IntBuffer indexBuffer, IntBuffer symbolLengthBuffer, ByteBuffer symbolTableBuffer) {
    this.indexBuffer = indexBuffer;
    this.symbolLengthBuffer = symbolLengthBuffer;
    this.symbolTableBuffer = symbolTableBuffer;
  }

  @Override
  protected String getValueFromBuffer(int index) {
    if (decodedValues == null) {
      if (symbolLengthBuffer == null) {
        // TODO: change FsstEncoder to take offsets instead of length to get rid of this conversion
        symbolLengthBuffer = offsetToLengthBuffer(symbolTableOffsetBuffer);
        lengthBuffer = offsetToLengthBuffer(dictionaryOffsetBuffer);
      }

      // TODO: refactor -> get rid of that copy
      var symbolTable = new byte[symbolTableBuffer.limit() - symbolTableBuffer.position()];
      System.arraycopy(
          symbolTableBuffer.array(),
          symbolTableBuffer.position(),
          symbolTable,
          0,
          symbolTable.length);
      var data = new byte[dataBuffer.limit() - dataBuffer.position()];
      System.arraycopy(dataBuffer.array(), dataBuffer.position(), data, 0, data.length);
      byte[] dictionaryBuffer = FsstEncoder.decode(symbolTable, symbolLengthBuffer.array(), data);

      var decodedDictionary = new ArrayList<String>();
      var strStart = 0;
      for (var l : lengthBuffer.array()) {
        var v = Arrays.copyOfRange(dictionaryBuffer, strStart, strStart + l);
        decodedDictionary.add(new String(v, StandardCharsets.UTF_8));
        strStart += l;
      }

      var dictionaryOffsets = indexBuffer.array();
      decodedValues = new ArrayList<>(dictionaryOffsets.length);
      for (var dictionaryOffset : dictionaryOffsets) {
        var value = decodedDictionary.get(dictionaryOffset);
        decodedValues.add(value);
      }
    }

    return decodedValues.get(index);
  }

  // TODO: get rid of that conversion
  public static IntBuffer offsetToLengthBuffer(IntBuffer offsetBuffer) {
    var lengthBuffer = IntBuffer.allocate(offsetBuffer.capacity() - 1);
    var previousOffset = offsetBuffer.get(0);
    for (var i = 1; i < offsetBuffer.capacity(); i++) {
      var offset = offsetBuffer.get(i);
      lengthBuffer.put(i - 1, offset - previousOffset);
      previousOffset = offset;
    }
    return lengthBuffer;
  }
}
