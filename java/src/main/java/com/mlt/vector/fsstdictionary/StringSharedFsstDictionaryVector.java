package com.mlt.vector.fsstdictionary;

import static com.mlt.vector.fsstdictionary.StringFsstDictionaryVector.offsetToLengthBuffer;

import com.mlt.converter.encodings.fsst.FsstEncoder;
import com.mlt.vector.VariableSizeVector;
import com.mlt.vector.dictionary.DictionaryDataVector;
import java.nio.ByteBuffer;
import java.nio.IntBuffer;
import java.nio.charset.StandardCharsets;
import java.util.*;

public class StringSharedFsstDictionaryVector extends VariableSizeVector<Map<String, String>> {
  // TODO: extend from StringFsstDictionaryVector

  private IntBuffer symbolLengthBuffer;
  private final ByteBuffer symbolTableBuffer;
  private IntBuffer symbolOffsetBuffer;
  private IntBuffer offsetBuffer;
  private List<String> decodedDictionary;

  private final DictionaryDataVector[] dictionaryDataVectors;
  private Map<String, DictionaryDataVector> decodedDictionaryDataVectors;

  public StringSharedFsstDictionaryVector(
      String name,
      IntBuffer lengthBuffer,
      ByteBuffer dictionaryBuffer,
      IntBuffer symbolLengthBuffer,
      ByteBuffer symbolTableBuffer,
      DictionaryDataVector[] dictionaryDataVectors) {
    // TODO: refactor -> proper solution for StringSharedFsstDictionaryVector and size
    super(name, lengthBuffer, dictionaryBuffer, 0);
    this.dictionaryDataVectors = dictionaryDataVectors;
    this.symbolLengthBuffer = symbolLengthBuffer;
    this.symbolTableBuffer = symbolTableBuffer;
  }

  public static StringSharedFsstDictionaryVector createFromOffsetBuffer(
      String name,
      IntBuffer offsetBuffer,
      ByteBuffer dictionaryBuffer,
      IntBuffer symbolOffsetBuffer,
      ByteBuffer symbolTableBuffer,
      DictionaryDataVector[] dictionaryDataVectors) {
    var vector =
        new StringSharedFsstDictionaryVector(
            name, null, dictionaryBuffer, null, symbolTableBuffer, dictionaryDataVectors);
    vector.symbolOffsetBuffer = symbolOffsetBuffer;
    vector.offsetBuffer = offsetBuffer;
    return vector;
  }

  @Override
  protected Map<String, String> getValueFromBuffer(int index) {
    var values = new HashMap<String, String>();
    Arrays.stream(dictionaryDataVectors)
        .forEach(vector -> values.put(vector.name(), getPresentValue(vector.name(), index)));
    return values;
  }

  private String getPresentValue(String name, int index) {
    if (decodedDictionary == null) {
      decodeDictionary();
    }

    var dataVector = decodedDictionaryDataVectors.get(name);
    if (!dataVector.nullabilityBuffer().get(index)) {
      return null;
    }

    var indexBuffer = dataVector.offsetBuffer();
    var offset = indexBuffer.get(index);
    return decodedDictionary.get(offset);
  }

  private void decodeDictionary() {
    if (symbolLengthBuffer == null) {
      // TODO: change FsstEncoder to take offsets instead of length to get rid of this conversion
      symbolLengthBuffer = offsetToLengthBuffer(symbolOffsetBuffer);
      lengthBuffer = offsetToLengthBuffer(offsetBuffer);
    }

    if (this.decodedDictionaryDataVectors == null) {
      createDataVectorsMap();
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

    // TODO: refactor
    @SuppressWarnings("deprecation")
    byte[] dictionaryBuffer = FsstEncoder.decode(symbolTable, symbolLengthBuffer.array(), data);

    var decodedDictionary = new ArrayList<String>();
    var strStart = 0;
    for (var l : lengthBuffer.array()) {
      var v = Arrays.copyOfRange(dictionaryBuffer, strStart, strStart + l);
      decodedDictionary.add(new String(v, StandardCharsets.UTF_8));
      strStart += l;
    }

    this.decodedDictionary = decodedDictionary;
  }

  private void createDataVectorsMap() {
    decodedDictionaryDataVectors = new HashMap<>(dictionaryDataVectors.length);
    for (var dictionaryDataVector : dictionaryDataVectors) {
      decodedDictionaryDataVectors.put(dictionaryDataVector.name(), dictionaryDataVector);
    }
  }
}
