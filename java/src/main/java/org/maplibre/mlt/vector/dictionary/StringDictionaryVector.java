package org.maplibre.mlt.vector.dictionary;

import org.maplibre.mlt.vector.BitVector;
import org.maplibre.mlt.vector.VariableSizeVector;
import java.nio.ByteBuffer;
import java.nio.IntBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Iterator;
import me.lemire.integercompression.differential.Delta;
import org.apache.commons.lang3.ArrayUtils;

public class StringDictionaryVector extends VariableSizeVector<String> implements Iterable<String> {
  /** Specifies where a specific string starts in the data buffer for a given index */
  private int[] offsetBuffer;

  /** Index into the data (dictionary) buffer */
  private IntBuffer indexBuffer;

  private int[] lazyOffsetBuffer;

  public StringDictionaryVector(
      String name,
      IntBuffer indexBuffer,
      IntBuffer lengthBuffer,
      ByteBuffer dictionaryBuffer,
      int size) {
    super(name, lengthBuffer, dictionaryBuffer, size);
    this.indexBuffer = indexBuffer;
  }

  public StringDictionaryVector(
      String name,
      BitVector nullabilityBuffer,
      IntBuffer indexBuffer,
      IntBuffer lengthBuffer,
      ByteBuffer dictionaryBuffer) {
    super(name, nullabilityBuffer, lengthBuffer, dictionaryBuffer);
    this.indexBuffer = indexBuffer;
  }

  public static StringDictionaryVector createNonNullableVector(
      String name,
      IntBuffer indexBuffer,
      IntBuffer offsetBuffer,
      ByteBuffer dictionaryBuffer,
      int size) {
    var vector = new StringDictionaryVector(name, indexBuffer, null, dictionaryBuffer, size);
    vector.offsetBuffer = offsetBuffer.array();
    return vector;
  }

  public static StringDictionaryVector createNullableVector(
      String name,
      BitVector nullabilityBuffer,
      IntBuffer indexBuffer,
      IntBuffer offsetBuffer,
      ByteBuffer dictionaryBuffer) {
    var vector =
        new StringDictionaryVector(name, nullabilityBuffer, indexBuffer, null, dictionaryBuffer);
    vector.offsetBuffer = offsetBuffer.array();
    return vector;
  }

  /**
   * in the current implementation the length buffer of string dictionaries can be lazy evaluated in
   * contrast to the VertexDictionary
   */
  /*
   * filter query
   * -> equal
   * -> not equal
   *
   * evaluation
   *  -> filter criteria to ByteBuffer
   *  -> if Fsst encoded -> to Fsst ByteBuffer
   *       -> convert String to Utf8 ByteBuffer
   *       -> search for substrings in the symbol table
   *       -> replace substrings with index from symbol table
   * */
  @Override
  protected String getValueFromBuffer(int index) {
    if (offsetBuffer == null) {
      lengthToOffsetBuffer();
    }

    var offset = indexBuffer.get(index);
    var start = offsetBuffer[offset];
    var length = offsetBuffer[offset + 1] - start;
    var strBuffer = dataBuffer.slice(dataBuffer.position() + start, length);
    return StandardCharsets.UTF_8.decode(strBuffer).toString();
  }

  /**
   * First occurrence of offset values has to be in ascending order for example the first occurrence
   * of offset 0 has to be before offset 1
   */
  @Override
  public Iterator<String> iterator() {
    return new Iterator<>() {
      private int index = 0;
      private int maxValueLazyOffsetBuffer = 0;

      @Override
      public boolean hasNext() {
        return index < indexBuffer.capacity();
      }

      @Override
      public String next() {
        if (nullabilityBuffer != null && !nullabilityBuffer.get(index)) {
          return null;
        }

        if (offsetBuffer != null) {
          return getValueFromBuffer(index++);
        } else if (lazyOffsetBuffer == null) {
          lazyOffsetBuffer = new int[lengthBuffer.capacity() + 1];
          lazyOffsetBuffer[0] = 0;
        }

        /* IndexBuffer -> LazyOffsetBuffer/LengthBuffer -> DataBuffer */
        var dictionaryIndex = indexBuffer.get(index++);
        var dataBufferOffset = lazyOffsetBuffer[dictionaryIndex];
        var length = lengthBuffer.get(dictionaryIndex);

        if (dictionaryIndex == maxValueLazyOffsetBuffer) {
          lazyOffsetBuffer[dictionaryIndex + 1] = dataBufferOffset + length;
          maxValueLazyOffsetBuffer++;
        }

        var strBuffer = dataBuffer.slice(dataBuffer.position() + dataBufferOffset, length);
        return StandardCharsets.UTF_8.decode(strBuffer).toString();
      }
    };
  }

  public int size() {
    // TODO: change StringDictionaryVector to work with this encoding
    return this.nullabilityBuffer != null
        ? this.nullabilityBuffer.size()
        : this.indexBuffer.capacity();
  }

  private void lengthToOffsetBuffer() {
    // TODO: get rid of the array copy
    offsetBuffer = ArrayUtils.addAll(new int[] {0}, lengthBuffer.array());
    // TODO: add delta of delta encoding to do it in one pass
    Delta.fastinverseDelta(offsetBuffer, 0, offsetBuffer.length, 0);
  }
}
