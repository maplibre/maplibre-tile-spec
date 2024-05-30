package com.mlt.vector.flat;

import com.mlt.vector.BitVector;
import com.mlt.vector.VariableSizeVector;
import me.lemire.integercompression.differential.Delta;

import java.nio.ByteBuffer;
import java.nio.IntBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Iterator;

//string as ByteBuffer -> new String(buffer, StandardCharsets.US_ASCII);
//Or String as CharBuffer -> buffer.subequence(2, 10).toString() -> lazy evaluation for filtering
public class StringFlatVector extends VariableSizeVector<String> implements Iterable<String> {
    private IntBuffer offsetBuffer;

    public StringFlatVector(String name, IntBuffer lengthBuffer, ByteBuffer dataBuffer) {
        super(name, lengthBuffer, dataBuffer);
    }

    public StringFlatVector(String name, BitVector nullabilityBuffer, IntBuffer lengthBuffer, ByteBuffer dataBuffer) {
        super(name, nullabilityBuffer, lengthBuffer, dataBuffer);
    }

    public static StringFlatVector createFromOffsetBuffer(String name, BitVector nullabilityBuffer,
                                                    IntBuffer offsetBuffer, ByteBuffer dataBuffer){
        var vector = new StringFlatVector(name, nullabilityBuffer, null, dataBuffer);
        vector.offsetBuffer = offsetBuffer;
        return vector;
    }

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
        if (offsetBuffer == null){
            decodeLengthBuffer();
        }

        var start = offsetBuffer.get(index);
        var length = offsetBuffer.get(index + 1) - start;
        var strBuffer =  dataBuffer.slice(start, length).array();
        return new String(strBuffer, StandardCharsets.UTF_8);
    }

    private void decodeLengthBuffer(){
        Delta.fastinverseDelta(lengthBuffer.array());
        offsetBuffer = lengthBuffer;
    }

    @Override
    public Iterator<String> iterator() {
        return new Iterator<>() {
            private int index = 0;
            private int offset  = 0;

            @Override
            public boolean hasNext() {
                return index < lengthBuffer.capacity();
            }

            @Override
            public String next() {
                if(offsetBuffer != null){
                    return getValueFromBuffer(index++);
                }

                var length = lengthBuffer.get(index++);
                var strBuffer =  dataBuffer.slice(offset, length).array();
                offset += length;
                return new String(strBuffer, StandardCharsets.UTF_8);
            }
        };
    }
}
