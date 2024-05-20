package com.mlt.converter.mvt;

/*
 * In the converter it is currently possible to map a set of feature properties into a nested struct with a depth of one level.
 * For example a set of name:* feature properties like name:de and name:en can be mapped into a name struct.
 * This has the advantage that the dictionary (Shared Dictionary Encoding) can be shared among the nested columns.
 * */
public record ColumnMapping(String mvtPropertyPrefix, String mvtDelimiterSign, boolean useSharedDictionaryEncoding) {
}
