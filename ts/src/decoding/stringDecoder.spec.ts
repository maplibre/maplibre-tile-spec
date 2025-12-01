import { describe, it, expect, beforeEach, vi } from 'vitest';
import { type LogicalStreamType } from "../metadata/tile/logicalStreamType";
import * as IntegerStreamDecoder from "./integerStreamDecoder";
import { StreamMetadataDecoder } from "../metadata/tile/streamMetadataDecoder";
import { type StreamMetadata } from "../metadata/tile/streamMetadata";
import { LengthType } from "../metadata/tile/lengthType";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import { ScalarType } from "../metadata/tile/scalarType";
import type IntWrapper from "./intWrapper";
import { type Column } from "../metadata/tileset/tilesetMetadata";
import { StringDecoder } from "./stringDecoder";
import * as integerDecoder from "./integerDecodingUtils";

function createMockStreamMetadata(
    physicalStreamType: PhysicalStreamType,
    logicalStreamType: LogicalStreamType,
    byteLength: number
): StreamMetadata {
    return {
        physicalStreamType,
        logicalStreamType,
        byteLength,
        _physicalStreamType: physicalStreamType,
        _logicalStreamType: logicalStreamType,
        _logicalLevelTechnique1: undefined,
        _logicalLevelTechnique2: undefined,
    } as unknown as StreamMetadata;
}

function createMockChildField(
    name: string = 'fieldName',
    type: string = 'scalarField',
    physicalType: ScalarType = ScalarType.STRING
) {
    return {
        name,
        type,
        scalarField: { physicalType },
    };
}

function createMockColumn(name: string = 'testColumn', children: any[] = []): Column {
    return {
        name,
        complexType: {
            children: children.length > 0 ? children : [createMockChildField()],
        },
    } as unknown as Column;
}

function setupOffsetMock(initialValue: number = 0) {
    let offsetValue = initialValue;
    const mockOffset = {
        get: vi.fn(() => offsetValue),
        add: vi.fn((amount: number) => {
            offsetValue += amount;
        }),
    } as unknown as IntWrapper;
    return mockOffset;
}

/**
 * Setup StreamMetadataDecoder mock with a pool of metadata.
 * Cycles through the pool if more calls are made than metadata provided.
 */
function setupStreamMetadataDecodeMock(metadata: StreamMetadata[]): void {
    let callCount = 0;
    vi.spyOn(StreamMetadataDecoder, 'decode').mockImplementation(() => {
        const result = metadata[callCount % metadata.length];
        callCount++;
        return result;
    });
}

function setupLengthStreamDecodeMock(offsetBuffer: Int32Array, streamMetadata: StreamMetadata): void {
    vi.spyOn(IntegerStreamDecoder, 'decodeLengthStreamToOffsetBuffer')
        .mockImplementation((data, offset, metadata) => {
            offset.add(metadata.byteLength);
            return offsetBuffer;
        });
}

function setupVarintDecodeMock(value: number | number[] = 0): void {
    const values = Array.isArray(value) ? value : [value];
    let callCount = 0;
    vi.spyOn(integerDecoder, 'decodeVarintInt32' as any).mockImplementation(() => {
        const result = new Int32Array([values[callCount] ?? 0]);
        callCount++;
        return result;
    });
}

describe('decodePlainStringVector', () => {
    it('should return null when plainLengthStream is null', () => {
        const result = (StringDecoder as any).decodePlainStringVector('test', null, new Uint8Array([1, 2, 3]), null, null);
        expect(result).toBeNull();
    });

    it('should return null when plainDataStream is null', () => {
        const result = (StringDecoder as any).decodePlainStringVector('test', new Int32Array([0, 3]), null, null, null);
        expect(result).toBeNull();
    });

    it('should return StringDictionaryVector when offsetStream exists (non-nullable)', () => {
        const plainLengthStream = new Int32Array([0, 3, 7]);
        const plainDataStream = new Uint8Array([97, 98, 99, 100, 101, 102, 103]);
        const offsetStream = new Int32Array([0, 1]);

        const result = (StringDecoder as any).decodePlainStringVector('test', plainLengthStream, plainDataStream, offsetStream, null);

        expect(result).toBeDefined();
        expect(result.name).toBe('test');
    });

    it('should return StringDictionaryVector when offsetStream exists (nullable)', () => {
        const plainLengthStream = new Int32Array([0, 3, 7]);
        const plainDataStream = new Uint8Array([97, 98, 99, 100, 101, 102, 103]);
        const offsetStream = new Int32Array([0, 1]);
        const nullabilityBuffer = { size: () => 2, get: (i: number) => true } as any;

        const result = (StringDecoder as any).decodePlainStringVector('test', plainLengthStream, plainDataStream, offsetStream, nullabilityBuffer);

        expect(result).toBeDefined();
        expect(result.name).toBe('test');
    });

    it('should return StringDictionaryVector with sparse offset when nullability mismatch', () => {
        const plainLengthStream = new Int32Array([0, 3, 7]);
        const plainDataStream = new Uint8Array([97, 98, 99, 100, 101, 102, 103]);
        const nullabilityBuffer = {
            size: () => 3,
            get: (i: number) => i !== 1
        } as any;

        const result = (StringDecoder as any).decodePlainStringVector('test', plainLengthStream, plainDataStream, null, nullabilityBuffer);

        expect(result).toBeDefined();
        expect(result.name).toBe('test');
    });

    it('should return StringFlatVector (non-nullable)', () => {
        const plainLengthStream = new Int32Array([0, 3, 7]);
        const plainDataStream = new Uint8Array([97, 98, 99, 100, 101, 102, 103]);

        const result = (StringDecoder as any).decodePlainStringVector('test', plainLengthStream, plainDataStream, null, null);

        expect(result).toBeDefined();
        expect(result.name).toBe('test');
    });
});

describe('decodeSharedDictionary', () => {
    let mockData: Uint8Array;
    let mockOffset: IntWrapper;
    let mockColumn: Column;
    let numFeatures: number;

    beforeEach(() => {
        mockData = new Uint8Array(256);
        mockOffset = setupOffsetMock();
        mockColumn = createMockColumn('testColumn', [createMockChildField()]);
        numFeatures = 10;
        vi.clearAllMocks();
    });

    describe('basic dictionary stream decoding', () => {
        it('should decode LENGTH stream for dictionary offset buffer', () => {
            const lengthLogicalType = { lengthType: LengthType.DICTIONARY } as unknown as LogicalStreamType;
            const streamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 20);
            const dataLogicalType = { dictionaryType: DictionaryType.SHARED } as unknown as LogicalStreamType;
            const dataStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dataLogicalType, 50);
            const expectedOffsetBuffer = new Int32Array([0, 5, 10, 15, 20]);

            setupStreamMetadataDecodeMock([streamMetadata, dataStreamMetadata]);
            setupLengthStreamDecodeMock(expectedOffsetBuffer, streamMetadata);
            setupVarintDecodeMock(0);

            const result = StringDecoder.decodeSharedDictionary(mockData, mockOffset, mockColumn, numFeatures);

            expect(StreamMetadataDecoder.decode).toHaveBeenCalledWith(mockData, mockOffset);
            expect(IntegerStreamDecoder.decodeLengthStreamToOffsetBuffer).toHaveBeenCalled();
            expect(result).toBeDefined();
            expect(Array.isArray(result)).toBe(true);
        });

        it('should decode LENGTH stream for symbol offset buffer', () => {
            const lengthLogicalType = { lengthType: LengthType.SYMBOL } as unknown as LogicalStreamType;
            const streamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 20);
            const dataLogicalType = { dictionaryType: DictionaryType.SHARED } as unknown as LogicalStreamType;
            const dataStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dataLogicalType, 50);
            const expectedOffsetBuffer = new Int32Array([0, 3, 6, 9]);

            setupStreamMetadataDecodeMock([streamMetadata, dataStreamMetadata]);
            setupLengthStreamDecodeMock(expectedOffsetBuffer, streamMetadata);
            setupVarintDecodeMock(0);

            const result = StringDecoder.decodeSharedDictionary(mockData, mockOffset, mockColumn, numFeatures);

            expect(result).toBeDefined();
        });
    });

    describe('dictionary buffer decoding', () => {
        it('should decode SINGLE dictionary type DATA stream', () => {
            const lengthLogicalType = { lengthType: LengthType.DICTIONARY } as unknown as LogicalStreamType;
            const lengthStreamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 20);
            const dictionaryLogicalType = { dictionaryType: DictionaryType.SINGLE } as unknown as LogicalStreamType;
            const dictionaryStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 40);
            const offsetBuffer = new Int32Array([0, 10, 20, 40]);

            setupStreamMetadataDecodeMock([lengthStreamMetadata, dictionaryStreamMetadata]);
            setupLengthStreamDecodeMock(offsetBuffer, lengthStreamMetadata);
            setupVarintDecodeMock(0);

            const result = StringDecoder.decodeSharedDictionary(mockData, mockOffset, mockColumn, numFeatures);

            expect(mockOffset.add).toHaveBeenCalledWith(40);
            expect(result).toBeDefined();
        });

        it('should advance offset correctly through LENGTH and DATA streams', () => {
            const lengthLogicalType = { lengthType: LengthType.DICTIONARY } as unknown as LogicalStreamType;
            const lengthStreamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 20);
            const dictionaryLogicalType = { dictionaryType: DictionaryType.SINGLE } as unknown as LogicalStreamType;
            const dictionaryStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 40);
            const offsetBuffer = new Int32Array([0, 10, 20, 40]);

            setupStreamMetadataDecodeMock([lengthStreamMetadata, dictionaryStreamMetadata]);
            setupLengthStreamDecodeMock(offsetBuffer, lengthStreamMetadata);
            setupVarintDecodeMock(0);

            const result = StringDecoder.decodeSharedDictionary(mockData, mockOffset, mockColumn, numFeatures);

            expect(mockOffset.add).toHaveBeenNthCalledWith(1, 20);
            expect(mockOffset.add).toHaveBeenNthCalledWith(2, 40);
            expect(result).toBeDefined();
        });
    });

    describe('symbol table buffer decoding', () => {
        it('should decode symbol table buffer when dictionary type is not SINGLE or SHARED', () => {
            const symbolTableLogicalType = { dictionaryType: DictionaryType.NONE } as unknown as LogicalStreamType;
            const lengthStreamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, symbolTableLogicalType, 20);
            const symbolTableMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, symbolTableLogicalType, 35);
            const dictionaryLogicalType = { dictionaryType: DictionaryType.SHARED } as unknown as LogicalStreamType;
            const dictionaryDataMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 50);
            const offsetBuffer = new Int32Array([0, 10, 20]);

            setupStreamMetadataDecodeMock([lengthStreamMetadata, symbolTableMetadata, dictionaryDataMetadata]);
            setupLengthStreamDecodeMock(offsetBuffer, lengthStreamMetadata);
            setupVarintDecodeMock(0);

            const result = StringDecoder.decodeSharedDictionary(mockData, mockOffset, mockColumn, numFeatures);

            expect(mockOffset.add).toHaveBeenNthCalledWith(1, 20);
            expect(mockOffset.add).toHaveBeenNthCalledWith(2, 35);
            expect(mockOffset.add).toHaveBeenNthCalledWith(3, 50);
            expect(result).toBeDefined();
        });
    });

    describe('with propertyColumnNames filter', () => {
        it('should accept optional propertyColumnNames parameter', () => {
            const propertyColumnNames = new Set(['testColumn']);
            const lengthLogicalType = { lengthType: LengthType.DICTIONARY } as unknown as LogicalStreamType;
            const lengthStreamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 20);
            const dictionaryLogicalType = { dictionaryType: DictionaryType.SHARED } as unknown as LogicalStreamType;
            const dictionaryStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 40);
            const skipStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 15);
            const offsetBuffer = new Int32Array([0, 10, 20, 40]);

            // Provide metadata for: LENGTH, DATA (for dictionary), DATA (for skipColumn)
            setupStreamMetadataDecodeMock([lengthStreamMetadata, dictionaryStreamMetadata, skipStreamMetadata]);
            setupLengthStreamDecodeMock(offsetBuffer, lengthStreamMetadata);
            setupVarintDecodeMock([1, 1]); // First field has 1 stream, then 1 more stream to skip

            const result = StringDecoder.decodeSharedDictionary(
                mockData,
                mockOffset,
                mockColumn,
                numFeatures,
                propertyColumnNames
            );

            expect(result).toBeDefined();
        });

        it('should skip column when propertyColumnNames does not include column', () => {
            const propertyColumnNames = new Set(['someOtherColumn']);
            const lengthLogicalType = { lengthType: LengthType.DICTIONARY } as unknown as LogicalStreamType;
            const lengthStreamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 20);
            const dictionaryLogicalType = { dictionaryType: DictionaryType.SHARED } as unknown as LogicalStreamType;
            const dictionaryStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 40);
            const skipStream1 = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 15);
            const skipStream2 = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 25);
            const offsetBuffer = new Int32Array([0, 10, 20, 40]);

            // Provide metadata for: LENGTH, DATA (for dictionary), and 2 streams to skip
            setupStreamMetadataDecodeMock([lengthStreamMetadata, dictionaryStreamMetadata, skipStream1, skipStream2]);
            setupLengthStreamDecodeMock(offsetBuffer, lengthStreamMetadata);
            setupVarintDecodeMock([2, 2]); // 2 streams in first field, 2 streams to skip

            const result = StringDecoder.decodeSharedDictionary(
                mockData,
                mockOffset,
                mockColumn,
                numFeatures,
                propertyColumnNames
            );

            expect(result).toBeDefined();
        });
    });

    describe('offset management', () => {
        it('should correctly advance offset through multiple streams', () => {
            const lengthLogicalType = { lengthType: LengthType.DICTIONARY } as unknown as LogicalStreamType;
            const lengthStreamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 20);
            const dictionaryLogicalType = { dictionaryType: DictionaryType.SHARED } as unknown as LogicalStreamType;
            const dictionaryStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 100);
            const offsetBuffer = new Int32Array([0, 25, 50, 75, 100]);

            setupStreamMetadataDecodeMock([lengthStreamMetadata, dictionaryStreamMetadata]);
            setupLengthStreamDecodeMock(offsetBuffer, lengthStreamMetadata);
            setupVarintDecodeMock(0);

            StringDecoder.decodeSharedDictionary(mockData, mockOffset, mockColumn, numFeatures);

            expect(mockOffset.add).toHaveBeenCalledWith(100);
        });
    });

    describe('edge cases', () => {
        it('should handle minimum feature count', () => {
            const lengthLogicalType = { lengthType: LengthType.DICTIONARY } as unknown as LogicalStreamType;
            const lengthStreamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 4);
            const dictionaryLogicalType = { dictionaryType: DictionaryType.SHARED } as unknown as LogicalStreamType;
            const dictionaryStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 10);
            const offsetBuffer = new Int32Array([0, 10]);

            setupStreamMetadataDecodeMock([lengthStreamMetadata, dictionaryStreamMetadata]);
            setupLengthStreamDecodeMock(offsetBuffer, lengthStreamMetadata);
            setupVarintDecodeMock(0);

            const result = StringDecoder.decodeSharedDictionary(mockData, mockOffset, mockColumn, 1);

            expect(result).toBeDefined();
        });

        it('should handle large feature count', () => {
            const lengthLogicalType = { lengthType: LengthType.DICTIONARY } as unknown as LogicalStreamType;
            const lengthStreamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 1000);
            const dictionaryLogicalType = { dictionaryType: DictionaryType.SHARED } as unknown as LogicalStreamType;
            const dictionaryStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 5000);
            const largeOffsetBuffer = new Int32Array(10001);
            for (let i = 0; i < largeOffsetBuffer.length; i++) {
                largeOffsetBuffer[i] = i * 500;
            }

            setupStreamMetadataDecodeMock([lengthStreamMetadata, dictionaryStreamMetadata]);
            setupLengthStreamDecodeMock(largeOffsetBuffer, lengthStreamMetadata);
            setupVarintDecodeMock(0);

            const result = StringDecoder.decodeSharedDictionary(mockData, mockOffset, mockColumn, 10000);

            expect(result).toBeDefined();
        });

        it('should handle empty child fields list', () => {
            const emptyColumnMock = createMockColumn('emptyColumn', []);
            const lengthLogicalType = { lengthType: LengthType.DICTIONARY } as unknown as LogicalStreamType;
            const lengthStreamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 0);
            const dictionaryLogicalType = { dictionaryType: DictionaryType.SHARED } as unknown as LogicalStreamType;
            const dictionaryStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 0);
            const offsetBuffer = new Int32Array([0]);

            setupStreamMetadataDecodeMock([lengthStreamMetadata, dictionaryStreamMetadata]);
            setupLengthStreamDecodeMock(offsetBuffer, lengthStreamMetadata);
            setupVarintDecodeMock(0);

            expect(() => {
                StringDecoder.decodeSharedDictionary(mockData, mockOffset, emptyColumnMock, 0);
            }).not.toThrow();
        });
    });

    describe('stream count handling', () => {
        it('should skip columns with 0 streams', () => {
            const lengthLogicalType = { lengthType: LengthType.DICTIONARY } as unknown as LogicalStreamType;
            const lengthStreamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 20);
            const dictionaryLogicalType = { dictionaryType: DictionaryType.SHARED } as unknown as LogicalStreamType;
            const dictionaryStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 40);
            const offsetBuffer = new Int32Array([0, 10, 20, 40]);

            setupStreamMetadataDecodeMock([lengthStreamMetadata, dictionaryStreamMetadata]);
            setupLengthStreamDecodeMock(offsetBuffer, lengthStreamMetadata);
            setupVarintDecodeMock(0);

            const result = StringDecoder.decodeSharedDictionary(mockData, mockOffset, mockColumn, numFeatures);

            expect(result).toBeDefined();
        });

        it('should throw error for non-string scalar fields', () => {
            const childFieldNonString = createMockChildField('fieldName', 'scalarField', ScalarType.INT_32);
            const columnWithNonStringField = createMockColumn('testColumn', [childFieldNonString]);
            const lengthLogicalType = { lengthType: LengthType.DICTIONARY } as unknown as LogicalStreamType;
            const lengthStreamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 20);
            const dictionaryLogicalType = { dictionaryType: DictionaryType.SHARED } as unknown as LogicalStreamType;
            const dictionaryStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 40);
            const skipStream1 = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 15);
            const skipStream2 = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 25);
            const offsetBuffer = new Int32Array([0, 10, 20, 40]);

            setupStreamMetadataDecodeMock([lengthStreamMetadata, dictionaryStreamMetadata, skipStream1, skipStream2]);
            setupLengthStreamDecodeMock(offsetBuffer, lengthStreamMetadata);
            setupVarintDecodeMock([2, 2]); // 2 streams in field, 2 streams to skip

            expect(() => {
                StringDecoder.decodeSharedDictionary(mockData, mockOffset, columnWithNonStringField, numFeatures);
            }).toThrow('Currently only optional string fields are implemented for a struct.');
        });
    });

    describe('return value validation', () => {
        it('should return Vector array', () => {
            const lengthLogicalType = { lengthType: LengthType.DICTIONARY } as unknown as LogicalStreamType;
            const lengthStreamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 20);
            const dictionaryLogicalType = { dictionaryType: DictionaryType.SHARED } as unknown as LogicalStreamType;
            const dictionaryStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 40);
            const offsetBuffer = new Int32Array([0, 10, 20, 40]);

            setupStreamMetadataDecodeMock([lengthStreamMetadata, dictionaryStreamMetadata]);
            setupLengthStreamDecodeMock(offsetBuffer, lengthStreamMetadata);
            setupVarintDecodeMock(0);

            const result = StringDecoder.decodeSharedDictionary(mockData, mockOffset, mockColumn, numFeatures);

            expect(result).toBeInstanceOf(Array);
        });

        it('should not return null or undefined', () => {
            const lengthLogicalType = { lengthType: LengthType.DICTIONARY } as unknown as LogicalStreamType;
            const lengthStreamMetadata = createMockStreamMetadata(PhysicalStreamType.LENGTH, lengthLogicalType, 20);
            const dictionaryLogicalType = { dictionaryType: DictionaryType.SHARED } as unknown as LogicalStreamType;
            const dictionaryStreamMetadata = createMockStreamMetadata(PhysicalStreamType.DATA, dictionaryLogicalType, 40);
            const offsetBuffer = new Int32Array([0, 10, 20, 40]);

            setupStreamMetadataDecodeMock([lengthStreamMetadata, dictionaryStreamMetadata]);
            setupLengthStreamDecodeMock(offsetBuffer, lengthStreamMetadata);
            setupVarintDecodeMock(0);

            const result = StringDecoder.decodeSharedDictionary(mockData, mockOffset, mockColumn, numFeatures);

            expect(result).not.toBeNull();
            expect(result).not.toBeUndefined();
        });
    });
});
