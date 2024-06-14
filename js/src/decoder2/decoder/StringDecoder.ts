import { IntWrapper } from './IntWrapper';
// import { FsstEncoder } from 'converter/encodings/fsst/FsstEncoder';
import { StreamMetadata } from '../metadata/stream/StreamMetadata';
import { StreamMetadataDecoder } from '../metadata/stream/StreamMetadataDecoder';
import { FeatureTableSchema, TileSetMetadata } from "../../../src/decoder/mlt_tileset_metadata_pb";
import { IntegerDecoder } from './IntegerDecoder';
import { DecodingUtils } from './DecodingUtils';

export class StringDecoder {

    /*
     * String column layouts:
     * -> plain -> present, length, data
     * -> dictionary -> present, length, dictionary, data
     * -> fsst dictionary -> symbolTable, symbolLength, dictionary, length, present, data
     * */

    public static decode(
        data: Uint8Array, offset: IntWrapper, numStreams: number,
        presentStream: Uint8Array, numValues: number) {
        let dictionaryLengthStream: number[] = null;
        let offsetStream: number[] = null;
        let dataStream: Uint8Array = null;
        let dictionaryStream: Uint8Array = null;
        let symbolLengthStream: number[] = null;
        let symbolTableStream: Uint8Array = null;

        for (let i = 0; i < numStreams; i++) {
            const streamMetadata = StreamMetadataDecoder.decode(data, offset);
            switch (streamMetadata.physicalStreamType()) {
                case 'OFFSET': {
                    offsetStream = IntegerDecoder.decodeIntStream(data, offset, streamMetadata, false);
                    break;
                }
                case 'LENGTH': {
                    const ls = IntegerDecoder.decodeIntStream(data, offset, streamMetadata, false);
                    if (streamMetadata.logicalStreamType().lengthType() === 'DICTIONARY') {
                        dictionaryLengthStream = ls;
                    } else {
                        symbolLengthStream = ls;
                    }
                    break;
                }
                case 'DATA': {
                    const ds = data.slice(offset.get(), offset.get() + streamMetadata.byteLength());
                    offset.add(streamMetadata.byteLength());
                    if (streamMetadata.logicalStreamType().dictionaryType() === 'SINGLE') {
                        dictionaryStream = ds;
                    } else {
                        symbolTableStream = ds;
                    }
                    break;
                }
            }
        }

        if (symbolTableStream) {
            // const utf8Values = FsstEncoder.decode(symbolTableStream, new Int32Array(symbolLengthStream), dictionaryStream);
            // return this.decodeDictionary(presentStream, dictionaryLengthStream, utf8Values, offsetStream, numValues);
            throw new Error("Not implemented.");
        } else if (dictionaryStream) {
            return this.decodeDictionary(presentStream, dictionaryLengthStream, dictionaryStream, offsetStream, numValues);
        } else {
            throw new Error("Not implemented.");
            // return this.decodePlain(presentStream, dictionaryLengthStream, dataStream, numValues);
        }
    }

    private static decodeDictionary(
        presentStream: Uint8Array, lengthStream: number[], utf8Values: Uint8Array,
        dictionaryOffsets: number[], numValues: number
    ): string[] {
        const dictionary: string[] = [];
        let dictionaryOffset = 0;

        for (const length of lengthStream) {
            const value = new TextDecoder("utf-8").decode(utf8Values.slice(dictionaryOffset, dictionaryOffset + length));
            dictionary.push(value);
            dictionaryOffset += length;
        }

        const values: string[] = [];
        let offset = 0;

        for (let i = 0; i < numValues; i++) {
            const present = presentStream[i];
            if (present) {
                const value = dictionary[dictionaryOffsets[offset++]];
                values.push(value);
            } else {
                values.push(null);
            }
        }

        return values;
    }
}
