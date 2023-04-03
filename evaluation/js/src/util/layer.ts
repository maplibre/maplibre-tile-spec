import {Geometry} from "./geometry";

//TODO: handle also the other data types
export enum PropertyDataType {
    //TODO: use int, uint and sint for integer values
    /* Proto
    * -> Varint	-> int32, int64, uint32, uint64, sint32, sint64, bool, enum
    * -> use uint32 if the value cannot be negative
    * -> use sint32 if the value is pretty much as likely to be negative as not (for some fuzzy definition of "as likely to be")
    * -> use int32 if the value could be negative, but that's much less likely than the value being positive
    *    (for example, if the application sometimes uses -1 to indicate an error or 'unknown' value and this is a relatively uncommon situation)
    * */
    /* Varint encoding is used for the value */
    UnsignedInteger,
    /* Varint and Zig-zag encoding is used for the value */
    SignedInteger,
    Float,
    String,
    Boolean
}

export interface Feature {
    id: number;
    geometry: Geometry;
    properties: Map<string, unknown>;
}

export interface Layer {
    name: string;
    features: Feature[];
}

export interface LayerMetadata{
    name: string;
    numFeatures: number;
    propertyNames: string[];
    propertyTypes : PropertyDataType[];
}
