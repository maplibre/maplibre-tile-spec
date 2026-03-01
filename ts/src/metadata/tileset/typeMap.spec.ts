import { describe, expect, it } from "vitest";
import { type Column, ComplexType, LogicalScalarType, ScalarType } from "./tilesetMetadata";
import { decodeColumnType, isGeometryColumn, isLogicalIdColumn } from "./typeMap";

describe("typeMap helpers", () => {
    it("should decode ID type codes with longID mapped from bit 1", () => {
        const cases = [
            { typeCode: 0, longID: false },
            { typeCode: 1, longID: false },
            { typeCode: 2, longID: true },
            { typeCode: 3, longID: true },
        ];

        for (const { typeCode, longID } of cases) {
            const column = decodeColumnType(typeCode);

            expect(column).not.toBeNull();
            if (!column) {
                throw new Error(`Failed to decode type code ${typeCode}`);
            }
            expect(isLogicalIdColumn(column)).toBe(true);
            expect(column?.scalarType?.longID).toBe(longID);
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

    it("should detect geometry columns by metadata type, not by name", () => {
        const structNamedGeometryColumn = {
            name: "geometry",
            type: "complexType",
            complexType: {
                type: "physicalType",
                physicalType: ComplexType.STRUCT,
                children: [],
            },
        } as Column;

        const geometryAnyNameColumn = {
            name: "geom",
            type: "complexType",
            complexType: {
                type: "physicalType",
                physicalType: ComplexType.GEOMETRY,
                children: [],
            },
        } as Column;

        expect(isGeometryColumn(structNamedGeometryColumn)).toBe(false);
        expect(isGeometryColumn(geometryAnyNameColumn)).toBe(true);
    });
});
