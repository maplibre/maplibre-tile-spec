import FeatureTable from "../vector/featureTable";
import {SelectionVector} from "../vector/filter/selectionVector";
import {FlatSelectionVector} from "../vector/filter/flatSelectionVector";
import {SINGLE_PART_GEOMETRY_TYPE} from "../vector/geometry/geometryType";
import { ExpressionSpecification } from "@maplibre/maplibre-gl-style-spec";

const compoundExpressions = ["all", "any"];
const comparisonExpressions = ["==", "!=", ">=", "<=", ">", "<"];
const matchExpressions = ["in", "!in", "has", "!has", "none"];

export default function filter(featureTable: FeatureTable, expression: ExpressionSpecification): SelectionVector{
    if(!expression){
        //TODO: get rid of that workaround for performance reasons
        const selectionVector = new Array(featureTable.numFeatures);
        for(let i = 0; i < featureTable.numFeatures; i++){
            selectionVector[i] = i;
        }
        return new FlatSelectionVector(selectionVector);
    }

    if(isCompoundExpression(expression)){
        return executeCompoundExpression(featureTable, expression);
    }
    if(isComparisonExpression(expression)){
        return executeComparisonExpression(featureTable, expression);
    }

    if(isMatchExpression(expression)){
        return executeMatchExpression(featureTable, expression);
    }

    throw new Error(`Filter ${expression[0]} not supported.`);
}

function isCompoundExpression(expression: ExpressionSpecification): boolean{
    return compoundExpressions.includes(expression[0]);
}

function isComparisonExpression(expression: ExpressionSpecification): boolean{
    return comparisonExpressions.includes(expression[0]);
}

function isMatchExpression(expression: ExpressionSpecification): boolean{
    return matchExpressions.includes(expression[0]);
}

function executeCompoundExpression(featureTable: FeatureTable, expressionSpecification: ExpressionSpecification): SelectionVector {
    if(expressionSpecification[0] !== "all"){
        throw new Error("Specified type of CompoundExpression not supported (yet).");
    }

    let selectionVector = null;
    const numExpressions = expressionSpecification.length - 1;

    const geometryTypeExpressionIndex = expressionSpecification.findIndex(e => e[0] === "$type")
    if(geometryTypeExpressionIndex > 0){
        /* Move geometry type expression ($type) to the front as currently no filtering based on a SelectionVector is
        *  supported in a GeometryVector. Only one geometry type expression is currently supported. */
        const geometryTypeExpression = expressionSpecification.splice(geometryTypeExpressionIndex, 1)[0];
        expressionSpecification.unshift(geometryTypeExpression);
    }

    for(let i = 1; i <= numExpressions; i++){
        const expression = expressionSpecification[i] as ExpressionSpecification;
        if(isComparisonExpression(expression)) {
            selectionVector = executeComparisonExpression(featureTable, expression, selectionVector);
        }
        else if(isMatchExpression(expression)) {
            selectionVector = executeMatchExpression(featureTable, expression, selectionVector);
        }
        else{
            throw new Error("Expression not supported.");
        }

        if(selectionVector.limit === 0){
            return selectionVector;
        }
    }

    return selectionVector;
}

function executeMatchExpression(featureTable: FeatureTable, expression: ExpressionSpecification, selectionVector?: SelectionVector): SelectionVector{
    //TODO: get rid of any case
    const comparisonInstruction = expression[0] as any;
    const columnName = expression[1] as string;

    const propertyVector = featureTable.getPropertyVector(columnName);
    if(!propertyVector){
        if(comparisonInstruction === "!has"){
            //TODO: use SequenceSelectionVector
            return selectionVector ?? createSequenceSelectionVector(featureTable);
        }

        return new FlatSelectionVector([]);
    }

    switch (comparisonInstruction) {
        case "in": {
            const filterLiterals = expression.slice(2, expression.length);
            if (selectionVector) {
                propertyVector.matchSelected(filterLiterals, selectionVector);
                return selectionVector;
            }

            return propertyVector.match(filterLiterals);
        }
        //TODO: check if this expression gets called
        case "!in": {
            const filterLiterals = expression.slice(2, expression.length);
            if (selectionVector) {
                propertyVector.noneMatchSelected(filterLiterals, selectionVector);
                return selectionVector;
            }

            return propertyVector.noneMatch(filterLiterals);
        }
        case "has":
            //TODO: use SequenceSelectionVector
            return selectionVector ?? createSequenceSelectionVector(featureTable);
        case "!has":
            return new FlatSelectionVector([]);
        default:
            throw new Error("Specified match expression not supported (yet).")
    }
}

function createSequenceSelectionVector(featureTable: FeatureTable) {
    const selectionVector = new Array(featureTable.numFeatures);
    //TODO: use SequenceSelectionVector
    for (let i = 0; i < featureTable.numFeatures; i++) {
        selectionVector[i] = i;
    }
    return new FlatSelectionVector(selectionVector);
}

function executeComparisonExpression(featureTable: FeatureTable, expression: ExpressionSpecification, selectionVector?: SelectionVector): SelectionVector {
    const comparisonInstruction = expression[0];
    const columnName = expression[1] as string;
    const predicateValue = expression[2] as any;

    if(columnName === "$type" || columnName === "geometry-type") {
        if(comparisonInstruction === "!="){
            throw new Error("Specified filter not supported on GeometryVector (yet).");
        }

        const geometryType = getSingePartGeometryType(predicateValue);
        return featureTable.geometryVector.filter(geometryType);
    }

    const propertyVector = featureTable.getPropertyVector(columnName);
    if(!propertyVector){
        if(comparisonInstruction === "!="){
            return selectionVector ?? createSequenceSelectionVector(featureTable);
        }

        return new FlatSelectionVector([]);
    }

    switch(comparisonInstruction){
        case "==": {
            if(selectionVector){
                propertyVector.filterSelected(predicateValue, selectionVector);
                return selectionVector;
            }

            return propertyVector.filter(predicateValue);
        }
        case "!=": {
            if (selectionVector) {
                propertyVector.filterNotEqualSelected(predicateValue, selectionVector);
                return selectionVector;
            }

            return propertyVector.filterNotEqual(predicateValue);
        }
        case ">=": {
            if (selectionVector) {
                propertyVector.greaterThanOrEqualToSelected(predicateValue, selectionVector);
                return selectionVector;
            }

            return propertyVector.greaterThanOrEqualTo(predicateValue);
        }
        case "<=": {
            if (selectionVector) {
                propertyVector.smallerThanOrEqualToSelected(predicateValue, selectionVector);
                return selectionVector;
            }

            return propertyVector.smallerThanOrEqualTo(predicateValue);
        }
        default: {
            throw new Error("Comparison expression not supported.");
        }
    }
}

function getSingePartGeometryType(geometryType: string): SINGLE_PART_GEOMETRY_TYPE {
    switch (geometryType) {
        case "Point":
            return SINGLE_PART_GEOMETRY_TYPE.POINT;
        case "LineString":
            return SINGLE_PART_GEOMETRY_TYPE.LINESTRING;
        case "Polygon":
            return SINGLE_PART_GEOMETRY_TYPE.POLYGON;
        default:
            throw new Error("Invalid geometry type");
    }
}