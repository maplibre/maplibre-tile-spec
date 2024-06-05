'use strict'; // code generated by pbf v3.2.1

var ColumnScope = exports.ColumnScope = {
    "FEATURE": {
        "value": 0,
        "options": {}
    },
    "VERTEX": {
        "value": 1,
        "options": {}
    }
};

var ScalarType = exports.ScalarType = {
    "BOOLEAN": {
        "value": 0,
        "options": {}
    },
    "INT_8": {
        "value": 1,
        "options": {}
    },
    "UINT_8": {
        "value": 2,
        "options": {}
    },
    "INT_32": {
        "value": 3,
        "options": {}
    },
    "UINT_32": {
        "value": 4,
        "options": {}
    },
    "INT_64": {
        "value": 5,
        "options": {}
    },
    "UINT_64": {
        "value": 6,
        "options": {}
    },
    "FLOAT": {
        "value": 7,
        "options": {}
    },
    "DOUBLE": {
        "value": 8,
        "options": {}
    },
    "STRING": {
        "value": 9,
        "options": {}
    }
};

var ComplexType = exports.ComplexType = {
    "VEC_2": {
        "value": 0,
        "options": {}
    },
    "VEC_3": {
        "value": 1,
        "options": {}
    },
    "GEOMETRY": {
        "value": 2,
        "options": {}
    },
    "GEOMETRY_Z": {
        "value": 3,
        "options": {}
    },
    "LIST": {
        "value": 4,
        "options": {}
    },
    "MAP": {
        "value": 5,
        "options": {}
    },
    "STRUCT": {
        "value": 6,
        "options": {}
    }
};

var LogicalScalarType = exports.LogicalScalarType = {
    "TIMESTAMP": {
        "value": 0,
        "options": {}
    },
    "DATE": {
        "value": 1,
        "options": {}
    },
    "JSON": {
        "value": 2,
        "options": {}
    }
};

var LogicalComplexType = exports.LogicalComplexType = {
    "BINARY": {
        "value": 0,
        "options": {}
    },
    "RANGE_MAP": {
        "value": 1,
        "options": {}
    }
};

// TileSetMetadata ========================================

var TileSetMetadata = exports.TileSetMetadata = {};

TileSetMetadata.read = function (pbf, end) {
    return pbf.readFields(TileSetMetadata._readField, {version: 0, featureTables: [], name: "", description: "", attribution: "", minZoom: 0, maxZoom: 0, bounds: [], center: []}, end);
};
TileSetMetadata._readField = function (tag, obj, pbf) {
    if (tag === 1) obj.version = pbf.readVarint(true);
    else if (tag === 2) obj.featureTables.push(FeatureTableSchema.read(pbf, pbf.readVarint() + pbf.pos));
    else if (tag === 3) obj.name = pbf.readString();
    else if (tag === 4) obj.description = pbf.readString();
    else if (tag === 5) obj.attribution = pbf.readString();
    else if (tag === 6) obj.minZoom = pbf.readVarint(true);
    else if (tag === 7) obj.maxZoom = pbf.readVarint(true);
    else if (tag === 8) pbf.readPackedDouble(obj.bounds);
    else if (tag === 9) pbf.readPackedDouble(obj.center);
};
TileSetMetadata.write = function (obj, pbf) {
    if (obj.version) pbf.writeVarintField(1, obj.version);
    if (obj.featureTables) for (var i = 0; i < obj.featureTables.length; i++) pbf.writeMessage(2, FeatureTableSchema.write, obj.featureTables[i]);
    if (obj.name) pbf.writeStringField(3, obj.name);
    if (obj.description) pbf.writeStringField(4, obj.description);
    if (obj.attribution) pbf.writeStringField(5, obj.attribution);
    if (obj.minZoom) pbf.writeVarintField(6, obj.minZoom);
    if (obj.maxZoom) pbf.writeVarintField(7, obj.maxZoom);
    if (obj.bounds) pbf.writePackedDouble(8, obj.bounds);
    if (obj.center) pbf.writePackedDouble(9, obj.center);
};

// FeatureTableSchema ========================================

var FeatureTableSchema = exports.FeatureTableSchema = {};

FeatureTableSchema.read = function (pbf, end) {
    return pbf.readFields(FeatureTableSchema._readField, {name: "", columns: []}, end);
};
FeatureTableSchema._readField = function (tag, obj, pbf) {
    if (tag === 1) obj.name = pbf.readString();
    else if (tag === 2) obj.columns.push(Column.read(pbf, pbf.readVarint() + pbf.pos));
};
FeatureTableSchema.write = function (obj, pbf) {
    if (obj.name) pbf.writeStringField(1, obj.name);
    if (obj.columns) for (var i = 0; i < obj.columns.length; i++) pbf.writeMessage(2, Column.write, obj.columns[i]);
};

// Column ========================================

var Column = exports.Column = {};

Column.read = function (pbf, end) {
    return pbf.readFields(Column._readField, {name: "", nullable: false, columnScope: 0, scalarType: null, type: null, complexType: null}, end);
};
Column._readField = function (tag, obj, pbf) {
    if (tag === 1) obj.name = pbf.readString();
    else if (tag === 2) obj.nullable = pbf.readBoolean();
    else if (tag === 3) obj.columnScope = pbf.readVarint();
    else if (tag === 4) obj.scalarType = ScalarColumn.read(pbf, pbf.readVarint() + pbf.pos), obj.type = "scalarType";
    else if (tag === 5) obj.complexType = ComplexColumn.read(pbf, pbf.readVarint() + pbf.pos), obj.type = "complexType";
};
Column.write = function (obj, pbf) {
    if (obj.name) pbf.writeStringField(1, obj.name);
    if (obj.nullable) pbf.writeBooleanField(2, obj.nullable);
    if (obj.columnScope) pbf.writeVarintField(3, obj.columnScope);
    if (obj.scalarType) pbf.writeMessage(4, ScalarColumn.write, obj.scalarType);
    if (obj.complexType) pbf.writeMessage(5, ComplexColumn.write, obj.complexType);
};

// ScalarColumn ========================================

var ScalarColumn = exports.ScalarColumn = {};

ScalarColumn.read = function (pbf, end) {
    return pbf.readFields(ScalarColumn._readField, {physicalType: 0, type: null, logicalType: 0}, end);
};
ScalarColumn._readField = function (tag, obj, pbf) {
    if (tag === 4) obj.physicalType = pbf.readVarint(), obj.type = "physicalType";
    else if (tag === 5) obj.logicalType = pbf.readVarint(), obj.type = "logicalType";
};
ScalarColumn.write = function (obj, pbf) {
    if (obj.physicalType) pbf.writeVarintField(4, obj.physicalType);
    if (obj.logicalType) pbf.writeVarintField(5, obj.logicalType);
};

// ComplexColumn ========================================

var ComplexColumn = exports.ComplexColumn = {};

ComplexColumn.read = function (pbf, end) {
    return pbf.readFields(ComplexColumn._readField, {physicalType: 0, type: null, logicalType: 0, children: []}, end);
};
ComplexColumn._readField = function (tag, obj, pbf) {
    if (tag === 4) obj.physicalType = pbf.readVarint(), obj.type = "physicalType";
    else if (tag === 5) obj.logicalType = pbf.readVarint(), obj.type = "logicalType";
    else if (tag === 6) obj.children.push(Field.read(pbf, pbf.readVarint() + pbf.pos));
};
ComplexColumn.write = function (obj, pbf) {
    if (obj.physicalType) pbf.writeVarintField(4, obj.physicalType);
    if (obj.logicalType) pbf.writeVarintField(5, obj.logicalType);
    if (obj.children) for (var i = 0; i < obj.children.length; i++) pbf.writeMessage(6, Field.write, obj.children[i]);
};

// Field ========================================

var Field = exports.Field = {};

Field.read = function (pbf, end) {
    return pbf.readFields(Field._readField, {name: "", nullable: false, scalarField: null, type: null, complexField: null}, end);
};
Field._readField = function (tag, obj, pbf) {
    if (tag === 1) obj.name = pbf.readString();
    else if (tag === 2) obj.nullable = pbf.readBoolean();
    else if (tag === 3) obj.scalarField = ScalarField.read(pbf, pbf.readVarint() + pbf.pos), obj.type = "scalarField";
    else if (tag === 4) obj.complexField = ComplexField.read(pbf, pbf.readVarint() + pbf.pos), obj.type = "complexField";
};
Field.write = function (obj, pbf) {
    if (obj.name) pbf.writeStringField(1, obj.name);
    if (obj.nullable) pbf.writeBooleanField(2, obj.nullable);
    if (obj.scalarField) pbf.writeMessage(3, ScalarField.write, obj.scalarField);
    if (obj.complexField) pbf.writeMessage(4, ComplexField.write, obj.complexField);
};

// ScalarField ========================================

var ScalarField = exports.ScalarField = {};

ScalarField.read = function (pbf, end) {
    return pbf.readFields(ScalarField._readField, {physicalType: 0, type: null, logicalType: 0}, end);
};
ScalarField._readField = function (tag, obj, pbf) {
    if (tag === 1) obj.physicalType = pbf.readVarint(), obj.type = "physicalType";
    else if (tag === 2) obj.logicalType = pbf.readVarint(), obj.type = "logicalType";
};
ScalarField.write = function (obj, pbf) {
    if (obj.physicalType) pbf.writeVarintField(1, obj.physicalType);
    if (obj.logicalType) pbf.writeVarintField(2, obj.logicalType);
};

// ComplexField ========================================

var ComplexField = exports.ComplexField = {};

ComplexField.read = function (pbf, end) {
    return pbf.readFields(ComplexField._readField, {physicalType: 0, type: null, logicalType: 0, children: []}, end);
};
ComplexField._readField = function (tag, obj, pbf) {
    if (tag === 1) obj.physicalType = pbf.readVarint(), obj.type = "physicalType";
    else if (tag === 2) obj.logicalType = pbf.readVarint(), obj.type = "logicalType";
    else if (tag === 3) obj.children.push(Field.read(pbf, pbf.readVarint() + pbf.pos));
};
ComplexField.write = function (obj, pbf) {
    if (obj.physicalType) pbf.writeVarintField(1, obj.physicalType);
    if (obj.logicalType) pbf.writeVarintField(2, obj.logicalType);
    if (obj.children) for (var i = 0; i < obj.children.length; i++) pbf.writeMessage(3, Field.write, obj.children[i]);
};
