package com.covt.evaluation;

import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.MvtReader;
import io.github.sebasbaumh.mapbox.vectortile.adapt.jts.TagKeyValueMapConverter;
import org.locationtech.jts.geom.GeometryFactory;
import org.locationtech.jts.geom.PrecisionModel;
import org.locationtech.jts.geom.impl.PackedCoordinateSequenceFactory;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.sql.DriverManager;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.zip.GZIPInputStream;


public class MvtUtils {
    private static final String ID_KEY = "id";
    private static final String TILE_DATA_KEY = "tile_data";

    public static MapboxVectorTile getMVT(String mbTilesFileName, int zoom, int x, int y) throws SQLException, IOException, ClassNotFoundException {
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

        long start = System.nanoTime();
        //long start = System.currentTimeMillis();
        var inputStream = new ByteArrayInputStream(gzipCompressedMvt);
        var gZIPInputStream = new GZIPInputStream(inputStream);
        var mvtTile = gZIPInputStream.readAllBytes();
        //long elapsedTimeMillis = System.currentTimeMillis()-start;
        var elapsedTime = (System.nanoTime() - start) / Math.pow(10, 6);


        /*Path path = Paths.get("5_16_21.mvt");
        Files.write(path, mvtTile);*/

        var result = MvtReader.loadMvt(
                new ByteArrayInputStream(mvtTile),
                MvtUtils.createGeometryFactory(),
                new TagKeyValueMapConverter(false, "id"));
        final var mvtLayers = result.getLayers();

        var layers = new ArrayList<Layer>();
        for(var layer : mvtLayers){
            var name = layer.getName();
            var mvtFeatures = layer.getGeometries();
            var features = new ArrayList<Feature>();
            for(var mvtFeature : mvtFeatures){
                var properties = ((LinkedHashMap)mvtFeature.getUserData());
                var id = (long)properties.get(ID_KEY);
                properties.remove(ID_KEY);
                var feature = new Feature(id, mvtFeature, properties);
                features.add(feature);
            }

            layers.add(new Layer(name, features));
        }

        return new MapboxVectorTile(layers, gzipCompressedMvt.length, mvtTile.length);
    }

    private static GeometryFactory createGeometryFactory() {
        final PrecisionModel precisionModel = new PrecisionModel();
        final PackedCoordinateSequenceFactory coordinateSequenceFactory =
                new PackedCoordinateSequenceFactory(PackedCoordinateSequenceFactory.DOUBLE);
        return new GeometryFactory(precisionModel, 0, coordinateSequenceFactory);
    }

}
