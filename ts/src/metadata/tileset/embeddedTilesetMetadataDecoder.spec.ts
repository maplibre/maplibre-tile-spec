import { describe, expect, it } from "vitest";
import { decodeField, decodeEmbeddedTileSetMetadata } from "./embeddedTilesetMetadataDecoder";
import IntWrapper from "../../decoding/intWrapper";
import { concatenateBuffers } from "../../decoding/decodingTestUtils";
import { ComplexType, LogicalScalarType, ScalarType } from "./tilesetMetadata";
import {
    encodeChildCount,
    encodeFieldName,
    encodeTypeCode,
    scalarTypeCode,
} from "../../encoding/embeddedTilesetMetadataEncoder";

const STRUCT_TYPE_CODE = 30;

describe("embeddedTilesetMetadataDecoder", () => {
    describe("decodeField", () => {
        describe("scalar fields", () => {
            it("should decode non-nullable STRING field", () => {
                const buffer = concatenateBuffers(
                    encodeTypeCode(scalarTypeCode(ScalarType.STRING, false)),
                    encodeFieldName("street"),
                );
                const field = decodeField(buffer, new IntWrapper(0));

                expect(field.name).toBe("street");
                expect(field.nullable).toBe(false);
                expect(field.type).toBe("scalarField");
                expect(field.scalarField?.physicalType).toBe(ScalarType.STRING);
            });

            it("should decode nullable UINT_64 field", () => {
                const buffer = concatenateBuffers(
                    encodeTypeCode(scalarTypeCode(ScalarType.UINT_64, true)),
                    encodeFieldName("population"),
                );
                const field = decodeField(buffer, new IntWrapper(0));

                expect(field.name).toBe("population");
                expect(field.nullable).toBe(true);
                expect(field.type).toBe("scalarField");
                expect(field.scalarField?.physicalType).toBe(ScalarType.UINT_64);
            });

            it("should decode BOOLEAN field", () => {
                const buffer = concatenateBuffers(
                    encodeTypeCode(scalarTypeCode(ScalarType.BOOLEAN, false)),
                    encodeFieldName("isActive"),
                );
                const field = decodeField(buffer, new IntWrapper(0));

                expect(field.name).toBe("isActive");
                expect(field.nullable).toBe(false);
                expect(field.type).toBe("scalarField");
                expect(field.scalarField?.physicalType).toBe(ScalarType.BOOLEAN);
            });

            it("should decode non-nullable UINT_32 field", () => {
                const buffer = concatenateBuffers(
                    encodeTypeCode(scalarTypeCode(ScalarType.UINT_32, false)),
                    encodeFieldName("count"),
                );
                const field = decodeField(buffer, new IntWrapper(0));

                expect(field.name).toBe("count");
                expect(field.nullable).toBe(false);
                expect(field.type).toBe("scalarField");
                expect(field.scalarField?.physicalType).toBe(ScalarType.UINT_32);
            });

            it("should decode nullable FLOAT field", () => {
                const buffer = concatenateBuffers(
                    encodeTypeCode(scalarTypeCode(ScalarType.FLOAT, true)),
                    encodeFieldName("temperature"),
                );
                const field = decodeField(buffer, new IntWrapper(0));

                expect(field.name).toBe("temperature");
                expect(field.nullable).toBe(true);
                expect(field.type).toBe("scalarField");
                expect(field.scalarField?.physicalType).toBe(ScalarType.FLOAT);
            });
        });

        describe("complex fields", () => {
            it("should decode STRUCT field with nested children", () => {
                const children = [
                    {
                        typeCode: scalarTypeCode(ScalarType.STRING, false),
                        name: "street",
                        nullable: false,
                        physicalType: ScalarType.STRING,
                    },
                    {
                        typeCode: scalarTypeCode(ScalarType.UINT_32, true),
                        name: "zipcode",
                        nullable: true,
                        physicalType: ScalarType.UINT_32,
                    },
                ];

                const buffer = concatenateBuffers(
                    encodeTypeCode(STRUCT_TYPE_CODE),
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

                for (let i = 0; i < children.length; i++) {
                    const child = children[i];

                    expect(field.complexField?.children[i].name).toBe(child.name);
                    expect(field.complexField?.children[i].nullable).toBe(child.nullable);
                    expect(field.complexField?.children[i].scalarField?.physicalType).toBe(child.physicalType);
                }
            });
        });

        describe("deeply nested structures", () => {
            it("should decode 3-level nested STRUCT", () => {
                const leafChildren = [
                    { typeCode: scalarTypeCode(ScalarType.FLOAT, false), name: "lat" },
                    { typeCode: scalarTypeCode(ScalarType.FLOAT, false), name: "lon" },
                ];

                const buffer = concatenateBuffers(
                    // Parent STRUCT "location"
                    encodeTypeCode(STRUCT_TYPE_CODE),
                    encodeFieldName("location"),
                    encodeChildCount(1),
                    // Child STRUCT "address"
                    encodeTypeCode(STRUCT_TYPE_CODE),
                    encodeFieldName("address"),
                    encodeChildCount(1),
                    // Grandchild STRUCT "coordinates"
                    encodeTypeCode(STRUCT_TYPE_CODE),
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

                for (let i = 0; i < leafChildren.length; i++) {
                    const child = leafChildren[i];

                    expect(coordinates?.complexField?.children[i].name).toBe(child.name);
                    expect(coordinates?.complexField?.children[i].scalarField?.physicalType).toBe(ScalarType.FLOAT);
                }
            });
        });

        describe("offset tracking", () => {
            it("should correctly advance offset", () => {
                const buffer = concatenateBuffers(
                    encodeTypeCode(scalarTypeCode(ScalarType.STRING, false)),
                    encodeFieldName("test"),
                );
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
                }).toThrow("Unsupported field type code 999. Supported: 10-29(scalars), 30(STRUCT)");
            });
        });
    });

    describe("decodeEmbeddedTileSetMetadata", () => {
        it("should decode tileset with STRUCT column", () => {
            const buffer = concatenateBuffers(
                encodeFieldName(""),
                encodeTypeCode(4096),
                encodeChildCount(1),
                encodeTypeCode(STRUCT_TYPE_CODE),
                encodeFieldName("props"),
                encodeChildCount(1),
                encodeTypeCode(scalarTypeCode(ScalarType.STRING, false)),
                encodeFieldName("name"),
            );

            const [metadata, extent] = decodeEmbeddedTileSetMetadata(buffer, new IntWrapper(0));

            expect(extent).toBe(4096);
            expect(metadata.featureTables[0].name).toBe("");
            expect(metadata.featureTables[0].columns[0].complexType.children).toHaveLength(1);
        });

        it("should decode logical ID metadata with implicit id column name", () => {
            const typeCode = 3;
            const buffer = concatenateBuffers(
                encodeFieldName("layer"),
                encodeTypeCode(4096),
                encodeChildCount(1),
                encodeTypeCode(typeCode),
            );

            const [metadata] = decodeEmbeddedTileSetMetadata(buffer, new IntWrapper(0));
            const idColumn = metadata.featureTables[0].columns[0];

            expect(idColumn.name).toBe("id");
            expect(idColumn.nullable).toBe(true);
            expect(idColumn.type).toBe("scalarType");
            expect(idColumn.scalarType?.type).toBe("logicalType");
            expect(idColumn.scalarType?.logicalType).toBe(LogicalScalarType.ID);
            expect(idColumn.scalarType?.longID).toBe(true);
        });
    });
});
