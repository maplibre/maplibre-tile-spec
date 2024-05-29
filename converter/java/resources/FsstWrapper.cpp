#include <cstdint>
#include <iostream>
#include <ostream>
#include <vector>
#include <FsstWrapper.h>
#include <fsst.h>

struct SymbolTableStruct {
    unsigned char symbolLengths[255];
    unsigned long long symbols[255];
    int nSymbols;
    std::vector<unsigned char> compressedData;
};

SymbolTableStruct fsstCompress(std::vector<unsigned char> inputBytes) {
    unsigned long n = 1;
    auto **srcBuf = (uint8_t **) calloc(n, sizeof(uint8_t *));
    auto **dstBuf = (uint8_t **) calloc(n, sizeof(uint8_t *));
    auto *srcLen = (size_t *) calloc(n, sizeof(size_t));
    auto *dstLen = (size_t *) calloc(n, sizeof(size_t));

    srcBuf[0] = inputBytes.data();
    srcLen[0] = inputBytes.size();
    uint64_t before_size = inputBytes.size();

    unsigned char serialized_encoder_buf[FSST_MAXHEADER];
    fsst_encoder_t *encoder = fsst_create(n, srcLen, const_cast<const uint8_t **>(srcBuf), 0);
    fsst_export(encoder, serialized_encoder_buf);

    // the first 8 bytes of serialized_encoder_buf is where the version field is stored
    uint64_t version;
    memcpy(&version, serialized_encoder_buf, 8);

    // nSymbols is stored in the second byte from the right of version
    uint32_t nSymbols = (version >> 8) & 0xFF;

    uint8_t lenHisto[8];
    for(uint32_t i=0; i<8; i++)
        lenHisto[i] = serialized_encoder_buf[9+i];

    unsigned long output_buffer_size = 7 + 4 * before_size; //1024 * 1024 * 1024
    auto output_buffer = (uint8_t *) calloc(output_buffer_size, sizeof(uint8_t));

    fsst_compress(encoder, n, srcLen, const_cast<const uint8_t **>(srcBuf), output_buffer_size, output_buffer, dstLen, dstBuf);
    size_t compressedDataLength = *dstLen;

    fsst_decoder_t decoder;
    fsst_import(&decoder, serialized_encoder_buf);

    // Pack symbolTableStruct with relevant data
    SymbolTableStruct symbolTableStruct{};
    symbolTableStruct.nSymbols = nSymbols;
    memcpy(symbolTableStruct.symbolLengths, decoder.len, sizeof(decoder.len));
    memcpy(symbolTableStruct.symbols, decoder.symbol, sizeof(decoder.symbol));

    for (size_t i = 0; i < n; ++i) {
        for (size_t j = 0; j < compressedDataLength; ++j) {
            symbolTableStruct.compressedData.push_back(dstBuf[i][j]);
        }
    }

    fsst_destroy(encoder);

    return symbolTableStruct;
}

JNIEXPORT jobject JNICALL Java_com_mlt_converter_encodings_fsst_FsstEncoder_compress(JNIEnv* env, jclass cls, jbyteArray inputBytes) {
    jbyte *bytes = env->GetByteArrayElements(inputBytes, NULL);
    jsize length = env->GetArrayLength(inputBytes);

    std::vector<unsigned char> byteVector(bytes, bytes + length);

    // Don't forget to release the memory as it's a direct pointer to the java array.
    env->ReleaseByteArrayElements(inputBytes, bytes, 0);

    SymbolTableStruct result = fsstCompress(byteVector);
	
    // Convert symbolLengths array and symbols array
    jsize nSymbols = (jint)result.nSymbols;
    jintArray symbolLengthsArray = env->NewIntArray(nSymbols);
    jint* tempIntData = new jint[nSymbols];

    int totalSymbolLengths = 0;
    for(int i = 0; i < nSymbols; i++) {
        totalSymbolLengths += result.symbolLengths[i];
    }

    jbyteArray symbolsArray = env->NewByteArray(totalSymbolLengths);
    int offset = 0;
    for (int i = 0; i < nSymbols; i++) {
        tempIntData[i] = (jint) result.symbolLengths[i];
        env->SetByteArrayRegion(symbolsArray, offset, tempIntData[i], reinterpret_cast<const jbyte *>(&result.symbols[i]));
        offset += tempIntData[i];
    }

    env->SetIntArrayRegion(symbolLengthsArray, 0, nSymbols, tempIntData);
    delete[] tempIntData;

    // Convert compressedData to a Java byte array
	auto compressedDataLength = result.compressedData.size();
    jbyteArray compressedData = env->NewByteArray(compressedDataLength);
    env->SetByteArrayRegion(compressedData, 0, compressedDataLength, (jbyte*)&result.compressedData[0]);
	
    // Create the Java SymbolTable object
    jclass symbolTableClass = env->FindClass("com/mlt/converter/encodings/fsst/SymbolTable");		
    jmethodID symbolTableCtor = env->GetMethodID(symbolTableClass, "<init>", "([B[I[B)V");
    jobject javaSymbolTable = env->NewObject(symbolTableClass, symbolTableCtor, symbolsArray, symbolLengthsArray, compressedData);
	
    return javaSymbolTable;
}
