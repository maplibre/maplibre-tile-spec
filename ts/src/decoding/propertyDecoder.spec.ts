import {afterEach, describe, expect, it, vi} from 'vitest';
import {StreamMetadataDecoder} from "../metadata/tile/streamMetadataDecoder";
import IntegerStreamDecoder from "./integerStreamDecoder";
import {decodePropertyColumn} from "./propertyDecoder";
import {Column} from "../metadata/tileset/tilesetMetadata";
import {ScalarType} from "../metadata/tile/scalarType";
import IntWrapper from "./intWrapper";
import {IntFlatVector} from "../vector/flat/intFlatVector";
import {LongFlatVector} from "../vector/flat/longFlatVector";
import {IntSequenceVector} from "../vector/sequence/intSequenceVector";
import {LongSequenceVector} from "../vector/sequence/longSequenceVector";
import {IntConstVector} from "../vector/constant/intConstVector";
import {LongConstVector} from "../vector/constant/longConstVector";
import {VectorType} from "../vector/vectorType";
import {StringDecoder} from "./stringDecoder";
import BitVector from "../vector/flat/bitVector";
import * as decodingUtils from './decodingUtils';
import {BooleanFlatVector} from "../vector/flat/booleanFlatVector";
import {FloatFlatVector} from "../vector/flat/floatFlatVector";
import {DoubleFlatVector} from "../vector/flat/doubleFlatVector";
import {StringFlatVector} from "../vector/flat/stringFlatVector";
import { LogicalLevelTechnique } from "../metadata/tile/logicalLevelTechnique";

// Constants for test data
const TEST_DATA = {
    BYTE_LENGTH: 12,
    NUM_VALUES: 3,
    NULLABILITY_BYTE_LENGTH: 1,
    BUFFER_SIZE: 100
};

// Helper: Create column with specific configuration
function createColumn(scalarType: ScalarType, nullable: boolean = false): Column {
    return {
        name: 'age',
        nullable,
        columnScope: null,
        type: 'scalarType',
        scalarType: {
            longID: false,
            physicalType: scalarType,
            logicalType: null,
            type: 'physicalType'
        },
        complexType: null
    };
}

// Helper: Setup stream metadata mock
function mockStreamMetadata(
    byteLength: number = TEST_DATA.BYTE_LENGTH,
    numValues: number = TEST_DATA.NUM_VALUES
) {
    return {
        byteLength,
        numValues,
        logicalLevelTechnique1: 0,
        logicalLevelTechnique2: 0,
        physicalLevelTechnique: 0,
    } as any;
}

// Helper: Setup RLE stream metadata for sequence encoding
function mockRleStreamMetadata(
    byteLength: number = TEST_DATA.BYTE_LENGTH,
    numValues: number = TEST_DATA.NUM_VALUES,
    numRleValues: number = 2
) {
    return {
        byteLength,
        numValues,
        numRleValues,
        logicalLevelTechnique1: 0,
        logicalLevelTechnique2: 0,
        physicalLevelTechnique: 0,
    } as any;
}

// Helper: Mock integer decoders (INT_32 or INT_64)
function mockIntegerDecoder(scalarType: ScalarType) {
    vi.spyOn(IntegerStreamDecoder, 'getVectorType')
        .mockReturnValue(VectorType.FLAT);

    if (scalarType === ScalarType.INT_64 || scalarType === ScalarType.UINT_64) {
        vi.spyOn(IntegerStreamDecoder, 'decodeLongStream')
            .mockReturnValue(new BigInt64Array([100n, 200n, 300n]));
    } else {
        vi.spyOn(IntegerStreamDecoder, 'decodeIntStream')
            .mockReturnValue(new Int32Array([100, 200, 300]));
    }
}

// Helper: Mock integer sequence decoders
function mockIntegerSequenceDecoder(scalarType: ScalarType) {
    vi.spyOn(IntegerStreamDecoder, 'getVectorType')
        .mockReturnValue(VectorType.SEQUENCE);

    if (scalarType === ScalarType.INT_64 || scalarType === ScalarType.UINT_64) {
        vi.spyOn(IntegerStreamDecoder, 'decodeSequenceLongStream')
            .mockReturnValue([10n, 20n]);
    } else {
        vi.spyOn(IntegerStreamDecoder, 'decodeSequenceIntStream')
            .mockReturnValue([10, 20]);
    }
}

// Helper: Mock integer const decoders
function mockIntegerConstDecoder(scalarType: ScalarType) {
    vi.spyOn(IntegerStreamDecoder, 'getVectorType')
        .mockReturnValue(VectorType.CONST);

    if (scalarType === ScalarType.INT_64 || scalarType === ScalarType.UINT_64) {
        vi.spyOn(IntegerStreamDecoder, 'decodeConstLongStream')
            .mockReturnValue(42n);
    } else {
        vi.spyOn(IntegerStreamDecoder, 'decodeConstIntStream')
            .mockReturnValue(42);
    }
}

// Helper: Mock float decoders (FLOAT or DOUBLE)
function mockFloatDecoder(scalarType: ScalarType) {
    if (scalarType === ScalarType.FLOAT) {
        vi.spyOn(decodingUtils, 'decodeFloatsLE')
            .mockReturnValue(new Float32Array([100.5, 200.5, 300.5]));
    } else if (scalarType === ScalarType.DOUBLE) {
        vi.spyOn(decodingUtils, 'decodeDoublesLE')
            .mockReturnValue(new Float64Array([100.5, 200.5, 300.5]));
    }
}

// Helper: Mock nullable float decoders
function mockNullableFloatDecoder(scalarType: ScalarType) {
    if (scalarType === ScalarType.FLOAT) {
        vi.spyOn(decodingUtils, 'decodeNullableFloatsLE')
            .mockReturnValue(new Float32Array([100.5, 200.5, 300.5]));
    } else if (scalarType === ScalarType.DOUBLE) {
        vi.spyOn(decodingUtils, 'decodeNullableDoublesLE')
            .mockReturnValue(new Float64Array([100.5, 200.5, 300.5]));
    }
}

// Helper: Mock nullable integer decoders
function mockNullableIntegerDecoder(scalarType: ScalarType) {
    vi.spyOn(IntegerStreamDecoder, 'getVectorType')
        .mockReturnValue(VectorType.FLAT);

    if (scalarType === ScalarType.INT_64 || scalarType === ScalarType.UINT_64) {
        vi.spyOn(IntegerStreamDecoder, 'decodeNullableLongStream')
            .mockReturnValue(new BigInt64Array([100n, 200n, 300n]));
    } else {
        vi.spyOn(IntegerStreamDecoder, 'decodeNullableIntStream')
            .mockReturnValue(new Int32Array([100, 200, 300]));
    }
}

// Helper: Setup nullable column with separate nullability stream
function setupNullableStreamMocks() {
    const metadataSpy = vi.spyOn(StreamMetadataDecoder, 'decode');

    // First call: nullability stream
    metadataSpy.mockReturnValueOnce({
        byteLength: TEST_DATA.NULLABILITY_BYTE_LENGTH,
        numValues: TEST_DATA.NUM_VALUES,
        logicalLevelTechnique1: 0,
        logicalLevelTechnique2: 0,
        physicalLevelTechnique: 0,
    } as any);

    // Subsequent calls: data stream
    metadataSpy.mockReturnValue(mockStreamMetadata());

    // Mock the nullability bitmap decoding
    vi.spyOn(decodingUtils, 'decodeBooleanRle')
        .mockReturnValue(new Uint8Array([0b00000111]));
}

describe('decodePropertyColumn', () => {
    afterEach(() => vi.restoreAllMocks());

    describe('Number Columns - Non-Nullable - Signed Types', () => {
        const numberTypes = [
            {
                scalarType: ScalarType.INT_32,
                vectorClass: IntFlatVector,
                mockFn: mockIntegerDecoder,
                testName: 'INT_32'
            },
            {
                scalarType: ScalarType.INT_64,
                vectorClass: LongFlatVector,
                mockFn: mockIntegerDecoder,
                testName: 'INT_64'
            }
        ];

        it.each(numberTypes)(
            'should decode $testName column',
            ({ scalarType, vectorClass, mockFn }) => {
                // Arrange
                vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
                mockFn(scalarType);
                const column = createColumn(scalarType, false);
                const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
                const offset = new IntWrapper(0);

                // Act
                const result = decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES);

                // Assert
                expect(result).toBeInstanceOf(vectorClass);
                expect((result as any)._name).toBe('age');
                expect((result as any).dataBuffer).toHaveLength(TEST_DATA.NUM_VALUES);
            }
        );
    });

    describe('Number Columns - Non-Nullable - Unsigned Types', () => {
        const numberTypes = [
            {
                scalarType: ScalarType.UINT_32,
                vectorClass: IntFlatVector,
                mockFn: mockIntegerDecoder,
                testName: 'UINT_32'
            },
            {
                scalarType: ScalarType.UINT_64,
                vectorClass: LongFlatVector,
                mockFn: mockIntegerDecoder,
                testName: 'UINT_64'
            }
        ];

        it.each(numberTypes)(
            'should decode $testName column',
            ({ scalarType, vectorClass, mockFn }) => {
                // Arrange
                vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
                mockFn(scalarType);
                const column = createColumn(scalarType, false);
                const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
                const offset = new IntWrapper(0);

                // Act
                const result = decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES);

                // Assert
                expect(result).toBeInstanceOf(vectorClass);
                expect((result as any)._name).toBe('age');
                expect((result as any).dataBuffer).toHaveLength(TEST_DATA.NUM_VALUES);
            }
        );
    });

    describe('Number Columns - Nullable - Signed Types', () => {
        const numberTypes = [
            { scalarType: ScalarType.INT_32, mockFn: mockNullableIntegerDecoder, testName: 'INT_32' },
            { scalarType: ScalarType.INT_64, mockFn: mockNullableIntegerDecoder, testName: 'INT_64' }
        ];

        it.each(numberTypes)(
            'should decode nullable $testName column with null mask',
            ({ scalarType, mockFn }) => {
                // Arrange
                setupNullableStreamMocks();
                mockFn(scalarType);
                const column = createColumn(scalarType, true);
                const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
                const offset = new IntWrapper(0);

                // Act
                const result = decodePropertyColumn(data, offset, column, 2, TEST_DATA.NUM_VALUES);

                // Assert
                expect(result).toBeDefined();
                expect((result as any)._name).toBe('age');
            }
        );
    });

    describe('Number Columns - Nullable - Unsigned Types', () => {
        const numberTypes = [
            { scalarType: ScalarType.UINT_32, mockFn: mockNullableIntegerDecoder, testName: 'UINT_32' },
            { scalarType: ScalarType.UINT_64, mockFn: mockNullableIntegerDecoder, testName: 'UINT_64' }
        ];

        it.each(numberTypes)(
            'should decode nullable $testName column with null mask',
            ({ scalarType, mockFn }) => {
                // Arrange
                setupNullableStreamMocks();
                mockFn(scalarType);
                const column = createColumn(scalarType, true);
                const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
                const offset = new IntWrapper(0);

                // Act
                const result = decodePropertyColumn(data, offset, column, 2, TEST_DATA.NUM_VALUES);

                // Assert
                expect(result).toBeDefined();
                expect((result as any)._name).toBe('age');
            }
        );
    });

    describe('Integer Vector Encoding Types - SEQUENCE', () => {
        const numberTypes = [
            {
                scalarType: ScalarType.INT_32,
                vectorClass: IntSequenceVector,
                mockFn: mockIntegerSequenceDecoder,
                testName: 'INT_32'
            },
            {
                scalarType: ScalarType.INT_64,
                vectorClass: LongSequenceVector,
                mockFn: mockIntegerSequenceDecoder,
                testName: 'INT_64'
            }
        ];

        it.each(numberTypes)(
            'should decode $testName with SEQUENCE encoding',
            ({ scalarType, vectorClass, mockFn }) => {
                // Arrange
                vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockRleStreamMetadata());
                mockFn(scalarType);
                const column = createColumn(scalarType, false);
                const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
                const offset = new IntWrapper(0);

                // Act
                const result = decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES);

                // Assert
                expect(result).toBeInstanceOf(vectorClass);
                expect((result as any)._name).toBe('age');
            }
        );
    });

    describe('Integer Vector Encoding Types - CONST', () => {
        const numberTypes = [
            {
                scalarType: ScalarType.INT_32,
                vectorClass: IntConstVector,
                mockFn: mockIntegerConstDecoder,
                testName: 'INT_32'
            },
            {
                scalarType: ScalarType.INT_64,
                vectorClass: LongConstVector,
                mockFn: mockIntegerConstDecoder,
                testName: 'INT_64'
            }
        ];

        it.each(numberTypes)(
            'should decode $testName with CONST encoding',
            ({ scalarType, vectorClass, mockFn }) => {
                // Arrange
                vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
                mockFn(scalarType);
                const column = createColumn(scalarType, false);
                const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
                const offset = new IntWrapper(0);

                // Act
                const result = decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES);

                // Assert
                expect(result).toBeInstanceOf(vectorClass);
                expect((result as any)._name).toBe('age');
            }
        );
    });

    describe('Float Columns - Non-Nullable', () => {
        const numberTypes = [
            {
                scalarType: ScalarType.FLOAT,
                vectorClass: FloatFlatVector,
                mockFn: mockFloatDecoder,
                testName: 'FLOAT'
            },
            {
                scalarType: ScalarType.DOUBLE,
                vectorClass: DoubleFlatVector,
                mockFn: mockFloatDecoder,
                testName: 'DOUBLE'
            }
        ];

        it.each(numberTypes)(
            'should decode $testName column',
            ({ scalarType, vectorClass, mockFn }) => {
                // Arrange
                vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
                mockFn(scalarType);
                const column = createColumn(scalarType, false);
                const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
                const offset = new IntWrapper(0);

                // Act
                const result = decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES);

                // Assert
                expect(result).toBeInstanceOf(vectorClass);
                expect((result as any)._name).toBe('age');
                expect((result as any).dataBuffer).toHaveLength(TEST_DATA.NUM_VALUES);
            }
        );
    });

    describe('Float Columns - Nullable', () => {
        const numberTypes = [
            {
                scalarType: ScalarType.FLOAT,
                vectorClass: FloatFlatVector,
                mockFn: mockNullableFloatDecoder,
                testName: 'FLOAT'
            },
            {
                scalarType: ScalarType.DOUBLE,
                vectorClass: DoubleFlatVector,
                mockFn: mockNullableFloatDecoder,
                testName: 'DOUBLE'
            }
        ];

        it.each(numberTypes)(
            'should decode nullable $testName column with null mask',
            ({ scalarType, vectorClass, mockFn }) => {
                // Arrange
                setupNullableStreamMocks();
                mockFn(scalarType);
                const column = createColumn(scalarType, true);
                const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
                const offset = new IntWrapper(0);

                // Act
                const result = decodePropertyColumn(data, offset, column, 2, TEST_DATA.NUM_VALUES);

                // Assert
                expect(result).toBeInstanceOf(vectorClass);
                expect((result as any)._name).toBe('age');
            }
        );
    });

    describe('Boolean Columns', () => {
        it('should decode non-nullable BOOLEAN column', () => {
            // Arrange
            vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
            vi.spyOn(decodingUtils, 'decodeBooleanRle')
                .mockReturnValue(new Uint8Array([0b00000111]));
            const column = createColumn(ScalarType.BOOLEAN, false);
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act
            const result = decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES);

            // Assert
            expect(result).toBeInstanceOf(BooleanFlatVector);
            expect((result as any)._name).toBe('age');
        });

        it('should decode nullable BOOLEAN column with null mask', () => {
            // Arrange
            setupNullableStreamMocks();
            vi.spyOn(decodingUtils, 'decodeNullableBooleanRle')
                .mockReturnValue(new Uint8Array([0b00000111]));
            const column = createColumn(ScalarType.BOOLEAN, true);
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act
            const result = decodePropertyColumn(data, offset, column, 2, TEST_DATA.NUM_VALUES);

            // Assert
            expect(result).toBeInstanceOf(BooleanFlatVector);
            expect((result as any)._name).toBe('age');
        });
    });

    describe('String Columns - Nullable', () => {
        const streamConfigs = [
            { totalStreams: 2, description: 'single data stream' },
            { totalStreams: 4, description: 'multiple data streams' }
        ];

        it.each(streamConfigs)(
            'should decode nullable STRING with $description',
            ({ totalStreams }) => {
                // Arrange
                setupNullableStreamMocks();
                const mockStringVector = { name: 'age' };
                const stringDecodeSpy = vi.spyOn(StringDecoder as any, 'decodeSharedDictionary')
                    .mockReturnValue(mockStringVector as any);
                const column = createColumn(ScalarType.STRING, true);
                const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
                const offset = new IntWrapper(0);

                // Act
                const result = decodePropertyColumn(data, offset, column, totalStreams, TEST_DATA.NUM_VALUES);

                // Assert
                expect((result as StringFlatVector).name).toBe(mockStringVector.name);
            }
        );
    });

    describe('Column Filtering', () => {
        it('should return null when column NOT in propertyColumnNames filter', () => {
            // Arrange
            vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
            const skipColumnSpy = vi.spyOn(decodingUtils, 'skipColumn');
            const column = createColumn(ScalarType.STRING);
            const filterList = new Set(['name', 'value']);
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act
            const result = decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES, filterList);

            // Assert
            expect(result).toBeNull();
            expect(skipColumnSpy).toHaveBeenCalledWith(1, data, offset);
        });

        it('should decode column when it IS in propertyColumnNames filter', () => {
            // Arrange
            vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
            vi.spyOn(decodingUtils, 'decodeBooleanRle')
                .mockReturnValue(new Uint8Array([0b00000111]));
            const column = createColumn(ScalarType.BOOLEAN);
            const filterList = new Set(['age', 'name']);
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act
            const result = decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES, filterList);

            // Assert
            expect(result).toBeInstanceOf(BooleanFlatVector);
        });

        it('should ignore filter when propertyColumnNames is undefined', () => {
            // Arrange
            vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
            vi.spyOn(decodingUtils, 'decodeBooleanRle')
                .mockReturnValue(new Uint8Array([0b00000111]));
            const column = createColumn(ScalarType.BOOLEAN);
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act
            const result = decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES, undefined);

            // Assert
            expect(result).toBeInstanceOf(BooleanFlatVector);
        });

        it('should handle empty filter set', () => {
            // Arrange
            vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
            const skipColumnSpy = vi.spyOn(decodingUtils, 'skipColumn');
            const column = createColumn(ScalarType.BOOLEAN);
            const filterList = new Set<string>();
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act
            const result = decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES, filterList);

            // Assert
            expect(result).toBeNull();
            expect(skipColumnSpy).toHaveBeenCalled();
        });
    });

    describe('Edge Cases', () => {
        it('should handle single value column', () => {
            // Arrange
            vi.spyOn(StreamMetadataDecoder, 'decode')
                .mockReturnValue(mockStreamMetadata(12, 1));
            vi.spyOn(IntegerStreamDecoder, 'getVectorType')
                .mockReturnValue(VectorType.FLAT);
            vi.spyOn(IntegerStreamDecoder, 'decodeIntStream')
                .mockReturnValue(new Int32Array([42]));
            const column = createColumn(ScalarType.INT_32);
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act
            const result = decodePropertyColumn(data, offset, column, 1, 1);

            // Assert
            expect(result).toBeInstanceOf(IntFlatVector);
            expect((result as any).dataBuffer).toHaveLength(1);
        });

        it('should handle large column with many values', () => {
            // Arrange
            const largeNumValues = 100000;
            vi.spyOn(StreamMetadataDecoder, 'decode')
                .mockReturnValue(mockStreamMetadata(400000, largeNumValues));
            vi.spyOn(IntegerStreamDecoder, 'getVectorType')
                .mockReturnValue(VectorType.FLAT);
            const largeArray = new Int32Array(largeNumValues);
            for (let i = 0; i < largeNumValues; i++) {
                largeArray[i] = i;
            }
            vi.spyOn(IntegerStreamDecoder, 'decodeIntStream')
                .mockReturnValue(largeArray);
            const column = createColumn(ScalarType.INT_32);
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act
            const result = decodePropertyColumn(data, offset, column, 1, largeNumValues);

            // Assert
            expect(result).toBeInstanceOf(IntFlatVector);
            expect((result as any).dataBuffer).toHaveLength(largeNumValues);
        });

        it('should handle zero numValues gracefully', () => {
            // Arrange
            vi.spyOn(StreamMetadataDecoder, 'decode')
                .mockReturnValue({...mockStreamMetadata(), numValues: 0});
            vi.spyOn(IntegerStreamDecoder, 'getVectorType')
                .mockReturnValue(VectorType.FLAT);
            vi.spyOn(IntegerStreamDecoder, 'decodeIntStream')
                .mockReturnValue(new Int32Array(0));
            const column = createColumn(ScalarType.INT_32);
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act
            const result = decodePropertyColumn(data, offset, column, 1, 0);

            // Assert
            expect(result).toBeInstanceOf(IntFlatVector);
            expect((result as any).dataBuffer).toHaveLength(0);
        });

        it('should handle multiple sequential columns with offset advancement', () => {
            // Arrange
            vi.spyOn(StreamMetadataDecoder, 'decode')
                .mockReturnValue(mockStreamMetadata(12, 3));
            vi.spyOn(IntegerStreamDecoder, 'getVectorType')
                .mockReturnValue(VectorType.FLAT);
            vi.spyOn(IntegerStreamDecoder, 'decodeIntStream')
                .mockReturnValue(new Int32Array([100, 200, 300]));
            const column1 = createColumn(ScalarType.INT_32);
            const column2 = createColumn(ScalarType.INT_32);
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act
            const result1 = decodePropertyColumn(data, offset, column1, 1, TEST_DATA.NUM_VALUES);
            const offsetAfterFirst = offset.get();
            const result2 = decodePropertyColumn(data, offset, column2, 1, TEST_DATA.NUM_VALUES);
            const offsetAfterSecond = offset.get();

            // Assert
            expect(result1).toBeInstanceOf(IntFlatVector);
            expect(result2).toBeInstanceOf(IntFlatVector);
            expect(offsetAfterSecond).toEqual(offsetAfterFirst);
        });

        it('should handle non-scalar column type returning null', () => {
            // Arrange
            const column: Column = {
                name: 'complex',
                nullable: false,
                columnScope: null,
                type: 'complexType',
                complexType: { type: 'arrayType' },
                scalarType: null
            } as any;
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act
            const result = decodePropertyColumn(data, offset, column, 2, TEST_DATA.NUM_VALUES);

            // Assert
            expect(result).toBeNull();
        });
    });

    describe('Offset Management', () => {

        it('should handle offset at non-zero starting position', () => {
            // Arrange
            vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
            vi.spyOn(IntegerStreamDecoder, 'getVectorType')
                .mockReturnValue(VectorType.FLAT);
            vi.spyOn(IntegerStreamDecoder, 'decodeIntStream')
                .mockReturnValue(new Int32Array([100, 200, 300]));

            const column = createColumn(ScalarType.INT_32);
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const startOffset = 50;
            const offset = new IntWrapper(startOffset);

            // Act
            const result = decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES);

            // Assert
            expect(result).toBeInstanceOf(IntFlatVector);
            expect(offset.get()).toEqual(startOffset);
        });

        it('should correctly skip columns with filterList and advance offset', () => {
            // Arrange
            vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
            const skipColumnSpy = vi.spyOn(decodingUtils, 'skipColumn')
                .mockImplementation((numStreams, data, offset) => {
                    offset.add(12 * numStreams); // Simulate skipping
                });

            const column = createColumn(ScalarType.INT_32);
            const filterList = new Set(['other_column']);
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);
            const startOffset = offset.get();

            // Act
            const result = decodePropertyColumn(data, offset, column, 3, TEST_DATA.NUM_VALUES, filterList);

            // Assert
            expect(result).toBeNull();
            expect(offset.get()).toBeGreaterThan(startOffset);
            expect(skipColumnSpy).toHaveBeenCalledWith(3, data, offset);
        });
    });

    describe('Type Consistency Checks', () => {
        it('should preserve column metadata in returned vector', () => {
            // Arrange
            vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
            vi.spyOn(IntegerStreamDecoder, 'getVectorType')
                .mockReturnValue(VectorType.FLAT);
            vi.spyOn(IntegerStreamDecoder, 'decodeIntStream')
                .mockReturnValue(new Int32Array([10, 20, 30]));

            const column = createColumn(ScalarType.INT_32);
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act
            const result = decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES);

            // Assert
            expect((result as any)._name).toBe(column.name);
        });

        it('should handle all signed and unsigned type combinations', () => {
            // Arrange
            const types = [
                ScalarType.INT_32, ScalarType.UINT_32,
                ScalarType.INT_64, ScalarType.UINT_64
            ];

            types.forEach(scalarType => {
                vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
                mockIntegerDecoder(scalarType);

                const column = createColumn(scalarType, false);
                const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
                const offset = new IntWrapper(0);

                // Act
                const result = decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES);

                // Assert
                expect(result).toBeDefined();
                expect((result as any)._name).toBe('age');

                vi.restoreAllMocks();
            });
        });
    });

    describe('Error Scenarios', () => {
        it('should handle invalid scalar type gracefully', () => {
            // Arrange
            vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
            const column = createColumn(999 as any); // Invalid type
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act & Assert
            expect(() => {
                decodePropertyColumn(data, offset, column, 1, TEST_DATA.NUM_VALUES);
            }).toThrow();
        });

        it('should handle mismatched numStreams for string type', () => {
            // Arrange
            vi.spyOn(StreamMetadataDecoder, 'decode').mockReturnValue(mockStreamMetadata());
            const column: Column = {
                name: 'stringCol',
                nullable: false,
                columnScope: null,
                type: 'stringType',
                scalarType: null,
                complexType: null
            } as any;
            const data = new Uint8Array(TEST_DATA.BUFFER_SIZE);
            const offset = new IntWrapper(0);

            // Act
            const result = decodePropertyColumn(data, offset, column, 2, TEST_DATA.NUM_VALUES);

            // Assert
            expect(result).toBeNull();
        });
    });
});
