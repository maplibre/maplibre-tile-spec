import { describe, expect, it } from "vitest";
import { type Column, ComplexType, LogicalScalarType, ScalarType } from "./tilesetMetadata";
import { decodeColumnType, isGeometryColumn, isLogicalIdColumn } from "./typeMap";

describe("typeMap helpers", () => {
    it("should decode ID type codes with logical metadata and bit flags", () => {
        const cases = [
            { typeCode: 0, nullable: false, longID: false },
            { typeCode: 1, nullable: true, longID: false },
            { typeCode: 2, nullable: false, longID: true },
            { typeCode: 3, nullable: true, longID: true },
        ];

        for (const { typeCode, nullable, longID } of cases) {
            const column = decodeColumnType(typeCode);

            expect(column).toMatchObject({
                nullable,
                type: "scalarType",
                scalarType: {
                    type: "logicalType",
                    logicalType: LogicalScalarType.ID,
                    longID,
                },
            });
            expect(isLogicalIdColumn(column)).toBe(true);
        }
    });

    it("should decode the GEOMETRY type code as a non-nullable complex column", () => {
        expect(decodeColumnType(4)).toMatchObject({
            nullable: false,
            type: "complexType",
            complexType: { type: "physicalType", physicalType: ComplexType.GEOMETRY },
        });
    });

    it("should decode the STRUCT type code as a non-nullable complex column", () => {
        expect(decodeColumnType(30)).toMatchObject({
            nullable: false,
            type: "complexType",
            complexType: { type: "physicalType", physicalType: ComplexType.STRUCT },
        });
    });

    it("should decode scalar type codes with the nullable flag in the low bit", () => {
        const cases: Array<{ even: number; physicalType: number }> = [
            { even: 10, physicalType: ScalarType.BOOLEAN },
            { even: 12, physicalType: ScalarType.INT_8 },
            { even: 14, physicalType: ScalarType.UINT_8 },
            { even: 16, physicalType: ScalarType.INT_32 },
            { even: 18, physicalType: ScalarType.UINT_32 },
            { even: 20, physicalType: ScalarType.INT_64 },
            { even: 22, physicalType: ScalarType.UINT_64 },
            { even: 24, physicalType: ScalarType.FLOAT },
            { even: 26, physicalType: ScalarType.DOUBLE },
            { even: 28, physicalType: ScalarType.STRING },
        ];

        for (const { even, physicalType } of cases) {
            expect(decodeColumnType(even)).toMatchObject({
                nullable: false,
                type: "scalarType",
                scalarType: { type: "physicalType", physicalType },
            });
            expect(decodeColumnType(even + 1)).toMatchObject({
                nullable: true,
                type: "scalarType",
                scalarType: { type: "physicalType", physicalType },
            });
        }
    });

    it("should return null for unsupported type codes", () => {
        expect(decodeColumnType(5)).toBeNull();
        expect(decodeColumnType(99)).toBeNull();
    });

    it("should return false for physical scalar columns even when column is named id", () => {
        const physicalIdNamedColumn = {
            name: "id",
            type: "scalarType",
            scalarType: {
                type: "physicalType",
                physicalType: ScalarType.UINT_32,
            },
        } as Column;

        expect(isLogicalIdColumn(physicalIdNamedColumn)).toBe(false);
    });

    it("should return true for logical ID columns regardless of column name", () => {
        const logicalIdAnyNameColumn = {
            name: "my_custom_id",
            type: "scalarType",
            scalarType: {
                type: "logicalType",
                logicalType: LogicalScalarType.ID,
                longID: true,
            },
        } as Column;

        expect(isLogicalIdColumn(logicalIdAnyNameColumn)).toBe(true);
    });

    it("should return false for STRUCT columns even when column is named geometry", () => {
        const structNamedGeometryColumn = {
            name: "geometry",
            type: "complexType",
            complexType: {
                type: "physicalType",
                physicalType: ComplexType.STRUCT,
                children: [],
            },
        } as Column;

        expect(isGeometryColumn(structNamedGeometryColumn)).toBe(false);
    });

    it("should return true for GEOMETRY columns regardless of column name", () => {
        const geometryAnyNameColumn = {
            name: "geom",
            type: "complexType",
            complexType: {
                type: "physicalType",
                physicalType: ComplexType.GEOMETRY,
                children: [],
            },
        } as Column;

        expect(isGeometryColumn(geometryAnyNameColumn)).toBe(true);
    });
});
