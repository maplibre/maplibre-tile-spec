import { VariableSizeVector } from "../variableSizeVector";
import type BitVector from "../flat/bitVector";
import { decodeFsst } from "../../encodings/fsstDecoder";
import { decodeString } from "../../encodings/decodingUtils";
import { type SelectionVector } from "../filter/selectionVector";

export class StringFsstDictionaryVector extends VariableSizeVector<Uint8Array, string> {
    private readonly textEncoder: TextEncoder;

    // TODO: extend from StringVector
    private symbolLengthBuffer: Uint32Array;
    private lengthBuffer: Uint32Array;
    private decodedDictionary: Uint8Array;

    constructor(
        name: string,
        private readonly indexBuffer: Int32Array,
        offsetBuffer: Int32Array,
        dictionaryBuffer: Uint8Array,
        private readonly symbolOffsetBuffer: Int32Array,
        private readonly symbolTableBuffer: Uint8Array,
        nullabilityBuffer: BitVector,
    ) {
        super(name, offsetBuffer, dictionaryBuffer, nullabilityBuffer);
        this.textEncoder = new TextEncoder();
    }

    filter(value: string): SelectionVector {
        /*
         * -> create utf-8 buffer from value
         * -> convert utf-8 buffer to fsst compressed corpus
         * -> replace utf-8 character with index of SymbolTable
         * */

        throw new Error("Not implemented yet.");
    }

    match(values: string[]): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    filterSelected(value: string, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    matchSelected(values: string[], selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    protected getValueFromBuffer(index: number): string {
        //if (this.decodedValues == null) {
        /*if (this.decodedDictionary == null) {
            if (this.symbolLengthBuffer == null) {
                // TODO: change FsstEncoder to take offsets instead of length to get rid of this conversion
                this.symbolLengthBuffer = this.offsetToLengthBuffer(this.symbolOffsetBuffer);
                this.lengthBuffer = this.offsetToLengthBuffer(this.offsetBuffer);
            }

            const dictionaryBuffer = decodeFsst(this.symbolTableBuffer, this.symbolLengthBuffer,
                this.dataBuffer);

            this.decodedDictionary = new Array<string>(this.lengthBuffer.length);
            let i = 0;
            let strStart = 0;
            for (const strLength of this.lengthBuffer) {
                this.decodedDictionary[i++] = decodeString(dictionaryBuffer, strStart, strStart + strLength);
                strStart += strLength;
            }

            /!*this.decodedValues = new Array(this.indexBuffer.length);
            i = 0;
            for (const index of this.indexBuffer) {
                const value = decodedDictionary[index];
                this.decodedValues[i++] = value;
            }*!/
        }*/
        /*this.decodedValues = new Array(this.indexBuffer.length);
            i = 0;
            for (const index of this.indexBuffer) {
                const value = decodedDictionary[index];
                this.decodedValues[i++] = value;
            }*/

        if (this.decodedDictionary == null) {
            if (this.symbolLengthBuffer == null) {
                // TODO: change FsstEncoder to take offsets instead of length to get rid of this conversion
                this.symbolLengthBuffer = this.offsetToLengthBuffer(this.symbolOffsetBuffer);
                this.lengthBuffer = this.offsetToLengthBuffer(this.offsetBuffer);
            }

            this.decodedDictionary = decodeFsst(this.symbolTableBuffer, this.symbolLengthBuffer, this.dataBuffer);
        }

        const offset = this.indexBuffer[index];
        const start = this.offsetBuffer[offset];
        const end = this.offsetBuffer[offset + 1];
        return decodeString(this.decodedDictionary, start, end);
    }

    // TODO: get rid of that conversion
    private offsetToLengthBuffer(offsetBuffer: Int32Array): Uint32Array {
        const lengthBuffer = new Uint32Array(offsetBuffer.length - 1);
        let previousOffset = offsetBuffer[0];
        for (let i = 1; i < offsetBuffer.length; i++) {
            const offset = offsetBuffer[i];
            lengthBuffer[i - 1] = offset - previousOffset;
            previousOffset = offset;
        }

        return lengthBuffer;
    }

    filterNotEqual(value: string): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    filterNotEqualSelected(value: string, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    greaterThanOrEqualTo(value: string): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    greaterThanOrEqualToSelected(value: string, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    noneMatch(values: string[]): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    noneMatchSelected(values: string[], selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    smallerThanOrEqualTo(value: string): SelectionVector {
        throw new Error("Not implemented yet.");
    }

    smallerThanOrEqualToSelected(value: string, selectionVector: SelectionVector): void {
        throw new Error("Not implemented yet.");
    }

    /*private getDictionaryIndex(value: string){
        const utf8Buffer = this.textEncoder.encode(value);
        for(let i = 0; i < utf8Buffer.length; i++){
            const codePoint = utf8Buffer[i];
            if(codePoint !== 255){
                return -1;
            }
        }
    }*/
}
