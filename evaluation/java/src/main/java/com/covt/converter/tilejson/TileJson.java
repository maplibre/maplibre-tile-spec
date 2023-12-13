package com.covt.converter.tilejson;

import com.fasterxml.jackson.annotation.JsonProperty;

import java.util.List;

public class TileJson {
    @JsonProperty("vector_layers")
    public List<VectorLayer> vectorLayers;
}
