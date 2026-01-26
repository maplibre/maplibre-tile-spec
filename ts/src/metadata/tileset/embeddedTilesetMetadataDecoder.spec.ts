import { describe, it, expect } from "vitest";
import { decodeField } from "./embeddedTilesetMetadataDecoder";
import IntWrapper from "../../decoding/intWrapper";
import { encodeVarintInt32Value } from "../../encoding/integerEncodingUtils";
import { concatenateBuffers } from "../../decoding/decodingTestUtils";
import { ScalarType, ComplexType } from "./tilesetMetadata";

/**
 * Encodes a single typeCode as a varint.
 */
function encodeTypeCode(typeCode: number): Uint8Array {
    const buffer = new Uint8Array(5);
    const offset = new IntWrapper(0);
    encodeVarintInt32Value(typeCode, buffer, offset);
    return buffer.slice(0, offset.get());
}

/**
 * Encodes a field name as a length-prefixed UTF-8 string.
 */
function encodeFieldName(name: string): Uint8Array {
    const textEncoder = new TextEncoder();
    const nameBytes = textEncoder.encode(name);
    const lengthBuf = new Uint8Array(5);
    const offset = new IntWrapper(0);
    encodeVarintInt32Value(nameBytes.length, lengthBuf, offset);
    const lengthSlice = lengthBuf.slice(0, offset.get());
    return concatenateBuffers(lengthSlice, nameBytes);
}

/**
 * Encodes a child count as a varint.
 */
function encodeChildCount(count: number): Uint8Array {
    const buffer = new Uint8Array(5);
    const offset = new IntWrapper(0);
    encodeVarintInt32Value(count, buffer, offset);
    return buffer.slice(0, offset.get());
}

describe("embeddedTilesetMetadataDecoder", () => {
    describe("decodeField", () => {
        describe("scalar fields", () => {
            it.each([
                { typeCode: 28, name: "street", nullable: false, physicalType: ScalarType.STRING, desc: "non-nullable STRING" },
                { typeCode: 23, name: "population", nullable: true, physicalType: ScalarType.UINT_64, desc: "nullable UINT_64" },
                { typeCode: 10, name: "isActive", nullable: false, physicalType: ScalarType.BOOLEAN, desc: "BOOLEAN" },
                { typeCode: 18, name: "count", nullable: false, physicalType: ScalarType.UINT_32, desc: "non-nullable UINT_32" },
                { typeCode: 25, name: "temperature", nullable: true, physicalType: ScalarType.FLOAT, desc: "nullable FLOAT" },
            ])("should decode $desc field", ({ typeCode, name, nullable, physicalType }) => {
                const buffer = concatenateBuffers(encodeTypeCode(typeCode), encodeFieldName(name));
                const field = decodeField(buffer, new IntWrapper(0));

                expect(field.name).toBe(name);
                expect(field.nullable).toBe(nullable);
                expect(field.type).toBe("scalarField");
                expect(field.scalarField?.physicalType).toBe(physicalType);
            });
        });

        describe("complex fields", () => {
            it("should decode STRUCT field with nested children", () => {
                const children = [
                    { typeCode: 28, name: "street", nullable: false, physicalType: ScalarType.STRING },
                    { typeCode: 19, name: "zipcode", nullable: true, physicalType: ScalarType.UINT_32 },
                ];

                const buffer = concatenateBuffers(
                    encodeTypeCode(30),
                    encodeFieldName("address"),
                    encodeChildCount(children.length),
                    ...children.flatMap((c) => [encodeTypeCode(c.typeCode), encodeFieldName(c.name)]),
                );

                const field = decodeField(buffer, new IntWrapper(0));

                expect(field.name).toBe("address");
                expect(field.nullable).toBe(false);
                expect(field.type).toBe("complexField");
                expect(field.complexField?.physicalType).toBe(ComplexType.STRUCT);
                expect(field.complexField?.children).toHaveLength(children.length);

                children.forEach((child, i) => {
                    expect(field.complexField?.children[i].name).toBe(child.name);
                    expect(field.complexField?.children[i].nullable).toBe(child.nullable);
                    expect(field.complexField?.children[i].scalarField?.physicalType).toBe(child.physicalType);
                });
            });
        });

        describe("deeply nested structures", () => {
            it("should decode 3-level nested STRUCT", () => {
                const leafChildren = [
                    { typeCode: 24, name: "lat" },
                    { typeCode: 24, name: "lon" },
                ];

                const buffer = concatenateBuffers(
                    // Parent STRUCT "location"
                    encodeTypeCode(30),
                    encodeFieldName("location"),
                    encodeChildCount(1),
                    // Child STRUCT "address"
                    encodeTypeCode(30),
                    encodeFieldName("address"),
                    encodeChildCount(1),
                    // Grandchild STRUCT "coordinates"
                    encodeTypeCode(30),
                    encodeFieldName("coordinates"),
                    encodeChildCount(leafChildren.length),
                    // Great-grandchildren
                    ...leafChildren.flatMap((c) => [encodeTypeCode(c.typeCode), encodeFieldName(c.name)]),
                );

                const field = decodeField(buffer, new IntWrapper(0));

                expect(field.name).toBe("location");
                expect(field.type).toBe("complexField");
                expect(field.complexField?.physicalType).toBe(ComplexType.STRUCT);

                const address = field.complexField?.children[0];
                expect(address?.name).toBe("address");
                expect(address?.type).toBe("complexField");

                const coordinates = address?.complexField?.children[0];
                expect(coordinates?.name).toBe("coordinates");
                expect(coordinates?.complexField?.children).toHaveLength(leafChildren.length);

                leafChildren.forEach((child, i) => {
                    expect(coordinates?.complexField?.children[i].name).toBe(child.name);
                    expect(coordinates?.complexField?.children[i].scalarField?.physicalType).toBe(ScalarType.FLOAT);
                });
            });
        });

        describe("offset tracking", () => {
            it("should correctly advance offset", () => {
                const buffer = concatenateBuffers(encodeTypeCode(28), encodeFieldName("test"));
                const offset = new IntWrapper(0);

                decodeField(buffer, offset);

                expect(offset.get()).toBe(buffer.length);
            });
        });

        describe("error handling", () => {
            it("should throw error for unsupported typeCode", () => {
                const buffer = encodeTypeCode(999);

                expect(() => {
                    decodeField(buffer, new IntWrapper(0));
                }).toThrow("Unsupported field type code: 999");
            });
        });
    });
});
