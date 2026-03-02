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

    it("should detect logical ID columns by metadata type, not by name", () => {
        const physicalIdNamedColumn = {
            name: "id",
            type: "scalarType",
            scalarType: {
                type: "physicalType",
                physicalType: ScalarType.UINT_32,
            },
        } as Column;

        const logicalIdAnyNameColumn = {
            name: "my_custom_id",
            type: "scalarType",
            scalarType: {
                type: "logicalType",
                logicalType: LogicalScalarType.ID,
                longID: true,
            },
        } as Column;

        expect(isLogicalIdColumn(physicalIdNamedColumn)).toBe(false);
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
