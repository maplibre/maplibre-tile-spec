package com.covt.evaluation.compression;

import org.apache.orc.PhysicalWriter;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.List;

public class TestOutputCatcher implements PhysicalWriter.OutputReceiver {
    int currentBuffer = 0;
    List<ByteBuffer> buffers = new ArrayList<>();

    @Override
    public void output(ByteBuffer buffer) throws IOException {
        buffers.add(buffer);
    }

    @Override
    public void suppress() {
    }

    public ByteBuffer getCurrentBuffer() {
        while (currentBuffer < buffers.size() && buffers.get(currentBuffer).remaining() == 0) {
            currentBuffer += 1;
        }
        return currentBuffer < buffers.size() ? buffers.get(currentBuffer) : null;
    }

    public byte[] getBuffer() throws IOException {
        //var buffer = this.buffers.stream().flatMap(b -> Stream.of(buffers.toArray())).toArray(Byte[]::new);

        ByteArrayOutputStream outputStream = new ByteArrayOutputStream( );
        for(var buffer : this.buffers){
            outputStream.write(buffer.array());
        }

        return outputStream.toByteArray();
    }

    public int getBufferSize(){
        var size = 0;
        for(var buffer : buffers){
            size  += buffer.array().length;
        }
        return size;
    }
}