package com.covt.converter.mvt;

import com.covt.converter.EncodingUtils;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.MvtReader;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.TagKeyValueMapConverter;
import no.ecc.vectortile.VectorTileDecoder;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.PrecisionModel;
import org.locationtech.jts.geom.impl.PackedCoordinateSequenceFactory;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.*;
import java.util.stream.Collectors;
import java.util.zip.GZIPInputStream;


public class MvtUtils {
    private static final String ID_KEY = "id";
    private static final String TILE_DATA_KEY = "tile_data";

    public static MapboxVectorTile decodeMvt(String mbTilesFileName, int zoom, int x, int y) throws SQLException, IOException, ClassNotFoundException {
        Class.forName("org.sqlite.JDBC");
        byte[] gzipCompressedMvt;
        try(var connection = DriverManager.getConnection("jdbc:sqlite:" + mbTilesFileName)) {
            try(var stmt = connection.createStatement()){
                try(ResultSet rs = stmt.executeQuery(String.format("SELECT %s FROM tiles WHERE tile_column = %d AND tile_row = %d AND zoom_level = %d;",
                        TILE_DATA_KEY, x, y, zoom))){
                    rs.next();
                    gzipCompressedMvt = rs.getBytes(TILE_DATA_KEY);
                }
            }
        }

        return  decodeMvt(unzip(gzipCompressedMvt), gzipCompressedMvt);
    }

    public static MapboxVectorTile decodeMvt(Path mvtFilePath) throws IOException {
        var mvt = Files.readAllBytes(mvtFilePath);
        return decodeMvt(mvt, EncodingUtils.gzipCompress(mvt));
    }

    public static MapboxVectorTile decodeMvt2(Path mvtFilePath) throws IOException {
        var mvt = Files.readAllBytes(mvtFilePath);

        var compressedMVT = EncodingUtils.gzipCompress(mvt);
        return decodeMvt2(mvt, compressedMVT);
    }

    public static byte[] unzip(byte[] buffer) throws IOException {
        try(var inputStream = new ByteArrayInputStream(buffer)){
            try(var gZIPInputStream = new GZIPInputStream(inputStream)){
                return gZIPInputStream.readAllBytes();
            }
        }
    }

    private static MapboxVectorTile decodeMvt(byte[] mvtTile, byte[] compressedMvt) throws IOException {
        var result = MvtReader.loadMvt(new ByteArrayInputStream(mvtTile), MvtUtils.createGeometryFactory(),
                new TagKeyValueMapConverter(true, ID_KEY));
        final var mvtLayers = result.getLayers();

        var layers = new ArrayList<Layer>();
        for(var layer : mvtLayers){
            var name = layer.getName();
            var mvtFeatures = layer.getGeometries();
            var features = new ArrayList<Feature>();
            for(var mvtFeature : mvtFeatures){
                var properties = ((LinkedHashMap<String, Object>)mvtFeature.getUserData());
                var id = (long)properties.get(ID_KEY);
                properties.remove(ID_KEY);
                var feature = new Feature(id, mvtFeature, properties);
                features.add(feature);
            }

            layers.add(new Layer(name, features));
        }

        var decompressionTime = getGzipDecompressionTime(compressedMvt);
        //TODO: evaluate tile extent
        return new MapboxVectorTile(layers, compressedMvt.length, mvtTile.length, 8192, decompressionTime);
    }

    private static MapboxVectorTile decodeMvt2(byte[] mvtTile, byte[] compressedMvt) throws IOException {
        VectorTileDecoder mvtDecoder = new VectorTileDecoder();
        mvtDecoder.setAutoScale(false);
        var tile = mvtDecoder.decode(mvtTile);
        var mvtFeatures = tile.asList();
        var layers = new ArrayList<Layer>();
        var tileExtent = 0;
        for(var layerName : tile.getLayerNames()){
            var layerFeatures =
                    mvtFeatures.stream().filter(f -> f.getLayerName().equals(layerName)).collect(Collectors.toList());

            var features = new ArrayList<Feature>();
            for(var mvtFeature : layerFeatures){
                var properties = new HashMap(mvtFeature.getAttributes());
                var id = 0l;
                if(properties.containsKey(ID_KEY)){
                    if(properties.get(ID_KEY) instanceof  String){
                        /* Rename the id property as it is a reserved column name in the COVT format */
                        properties.put("_" + ID_KEY, properties.get(ID_KEY));
                        properties.remove(ID_KEY);
                    }
                    else{
                        throw new RuntimeException("Only a string datatype for the id in the properties supported.");
                    }
                }

                var geometry = mvtFeature.getGeometry();
                var feature = new Feature(id, geometry, properties);
                features.add(feature);

                var featureTileExtent = mvtFeature.getExtent();
                if(featureTileExtent > tileExtent){
                    tileExtent = featureTileExtent;
                }
            }

            layers.add(new Layer(layerName, features));
        }


        var decompressionTime = getGzipDecompressionTime(compressedMvt);
        /* Start with one extent per tile not per layer like in MVT */
        return new MapboxVectorTile(layers, compressedMvt.length, mvtTile.length, tileExtent, decompressionTime);
    }

    private static double getGzipDecompressionTime(byte[] compressedMvt) throws IOException {
        long start = System.nanoTime();
        unzip(compressedMvt);
        long finish = System.nanoTime();
        return (finish - start) / 1_000_000d;
    }

    private static GeometryFactory createGeometryFactory() {
        final PrecisionModel precisionModel = new PrecisionModel();
        final PackedCoordinateSequenceFactory coordinateSequenceFactory =
                new PackedCoordinateSequenceFactory(PackedCoordinateSequenceFactory.DOUBLE);
        return new GeometryFactory(precisionModel, 0, coordinateSequenceFactory);
    }

}
