package com.mlt.converter.mvt;

import com.mlt.data.Layer;
import com.mlt.data.Feature;
import no.ecc.vectortile.VectorTileDecoder;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.PrecisionModel;
import org.locationtech.jts.geom.impl.PackedCoordinateSequenceFactory;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;
import java.util.stream.Collectors;

public class MvtUtils {

    public static MapboxVectorTile decodeMvt(Path mvtFilePath) throws IOException {
        var mvt = Files.readAllBytes(mvtFilePath);
        return decodeMvt(mvt);
    }

    public static MapboxVectorTile decodeMvt(byte[] mvtTile) throws IOException {
        VectorTileDecoder mvtDecoder = new VectorTileDecoder();
        mvtDecoder.setAutoScale(false);
        var tile = mvtDecoder.decode(mvtTile);
        var mvtFeatures = tile.asList();
        var layers = new ArrayList<Layer>();
        for(var layerName : tile.getLayerNames()){
            var layerFeatures =
                    mvtFeatures.stream().filter(f -> f.getLayerName().equals(layerName)).collect(Collectors.toList());
            var features = new ArrayList<Feature>();
            var tileExtent = 0;
            for(var mvtFeature : layerFeatures){
                //TODO: also transform properties
                var geometry = mvtFeature.getGeometry();
                //TODO: quick and dirty -> implement generic
                var transformedProperties = transformNestedPropertyNames(mvtFeature.getAttributes());
                var feature = new Feature(mvtFeature.getId(), geometry, transformedProperties);
                features.add(feature);
                var featureTileExtent = mvtFeature.getExtent();
                if(featureTileExtent > tileExtent){
                    tileExtent = featureTileExtent;
                }
            }

            layers.add(new Layer(layerName, features, tileExtent));
        }

        return new MapboxVectorTile(layers);
    }

    public static List<VectorTileDecoder.Feature> decodeMvtFast(byte[] mvtTile) throws IOException {
        VectorTileDecoder mvtDecoder = new VectorTileDecoder();
        mvtDecoder.setAutoScale(false);
        var decodedTile = mvtDecoder.decode(mvtTile);
        var features = decodedTile.asList();
        return features;
    }

    private static LinkedHashMap<String, Object> transformNestedPropertyNames(Map<String, Object> properties){
        var transformedProperties = new LinkedHashMap<String, Object>();
        properties.forEach((k ,v) -> transformedProperties.put(k.replace("_", ":"), v));
        return transformedProperties;
    }

}
