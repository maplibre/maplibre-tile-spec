package com.mlt.decoder;

import com.mlt.converter.ConversionConfig;
import com.mlt.converter.FeatureTableOptimizations;
import com.mlt.converter.MltConverter;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.converter.mvt.MapboxVectorTile;
import com.mlt.converter.mvt.MvtUtils;
import com.mlt.data.MapLibreTile;
import com.mlt.metadata.stream.PhysicalLevelTechnique;
import com.mlt.vector.FeatureTable;
import org.junit.jupiter.api.Test;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;

import static org.junit.jupiter.api.Assertions.assertEquals;

public class MltDecoderTest {
    private static final String OMT_MVT_PATH = Paths.get("..","..","test","fixtures","omt","mvt").toString();

    /** Decode tiles in an in-memory format optimized for sequential access */

    @Test
    public void decodeMlTile_Z2() throws IOException {
        var tileId = String.format("%s_%s_%s", 2, 2, 2);
        testTile(tileId);
    }

    @Test
    public void decodeMlTile_Z4() throws IOException {
        var tileId = String.format("%s_%s_%s", 4, 8, 10);
        testTile(tileId);

        var tileId2 = String.format("%s_%s_%s", 4, 3, 9);
        testTile(tileId2);
    }

    @Test
    public void decodeMlTile_Z5() throws IOException {
        var tileId = String.format("%s_%s_%s", 5, 16, 21);
        testTile(tileId);

        var tileId2 = String.format("%s_%s_%s", 5, 16, 20);
        testTile(tileId2);
    }

    @Test
    public void decodeMlTile_Z6() throws IOException {
        var tileId = String.format("%s_%s_%s", 6, 32, 41);
        //testTile(tileId);
    }

    @Test
    public void decodeMlTile_Z14() throws IOException {
        var tileId = String.format("%s_%s_%s", 14, 8298, 10748);
        //testTile(tileId);
    }

    private void testTile(String tileId) throws IOException {
        var mvtFilePath = Paths.get(OMT_MVT_PATH, tileId + ".mvt" );
        var mvTile = MvtUtils.decodeMvt(mvtFilePath);

        var columnMapping = new ColumnMapping("name", ":", true);
        var columnMappings = Optional.of(List.of(columnMapping));
        var tileMetadata = MltConverter.createTilesetMetadata(mvTile, columnMappings, true);

        var allowIdRegeneration = true;
        var allowSorting = false;
        //var allowIdRegeneration = true;
        //var allowSorting = true;
        var optimization = new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
        //TODO: fix -> either add columMappings per layer or global like when creating the scheme
        var optimizations = Map.of("place", optimization, "water_name", optimization, "transportation", optimization);
        var mlTile = MltConverter.convertMvt(mvTile, new ConversionConfig(true, false, optimizations),
                tileMetadata);

        var decodedTile = MltDecoder.decodeMlTile(mlTile, tileMetadata);

        compareTiles(decodedTile, mvTile);

        System.out.println("Ratio: " + Files.readAllBytes(mvtFilePath).length / (double)mlTile.length );
        System.out.println("Reduction: " + ((1 - (double)mlTile.length / Files.readAllBytes(mvtFilePath).length)*100));
    }

    public static void compareTiles(MapLibreTile mlTile, MapboxVectorTile mvTile){
        var mltLayers = mlTile.layers();
        var mvtLayers = mvTile.layers();

        System.out.println("---------------------------------------------------------------------------");
        for(var i = 0; i < mvtLayers.size(); i++){
            var mltLayer = mltLayers.get(i);
            var mvtLayer = mvtLayers.get(i);
            var mltFeatures = mltLayer.features();
            var mvtFeatures = mvtLayer.features();
            for(var j = 0; j < mvtFeatures.size(); j++){
                //var mltFeature = mltFeatures.get(j);
                var mvtFeature = mvtFeatures.get(j);
                var mltFeature = mltFeatures.stream().filter(f -> f.id() == mvtFeature.id()).findFirst().get();
                var mvtId = mvtFeature.id();
                var mltId = mltFeature.id();

                //System.out.println(mltLayer.name() + " " + j);
                //assertEquals(mvtId, mltId);

                /*try{
                    assertTrue(mltFeatures.stream().anyMatch(f -> f.geometry().equals(mvtFeature.geometry())));
                }
                catch(Error e){
                    System.out.println(e);
                }*/

                var mltGeometry = mltFeature.geometry();
                var mvtGeometry = mvtFeature.geometry();
                //assertEquals(mvtGeometry, mltGeometry);

                var mltProperties = mltFeature.properties();
                var mvtProperties = mvtFeature.properties();
                for(var mvtProperty : mvtProperties.entrySet()){
                    //if(mvtProperty.getKey().contains("name:ja") || mvtProperty.getKey().contains("name:ja:rm")){
                    if(mvtProperty.getKey().contains("name:ja:rm")){
                        System.out.println(mvtProperty.getKey() + " " + mvtProperty.getValue() + " " + mltProperties.get(mvtProperty.getKey()) + " " + j + " " + i);
                        continue;
                    }

                    var mltProperty = mltProperties.get(mvtProperty.getKey());
                    //assertEquals(mvtProperty.getValue(), mltProperty);
                    //System.out.println(mvtProperty.getKey() + ": " + mvtProperty.getValue() + ", " + mltProperty);
                }
            }
        }
    }

    /** decode tile in an in-memory format optimized for random access */

    @Test
    public void decodeMlTileVectorized_Z4() throws IOException {
        var tileId = String.format("%s_%s_%s", 4, 8, 10);
        //testTileVectorized(tileId);

        var tileId2 = String.format("%s_%s_%s", 4, 3, 9);
        //testTile(tileId2);
    }

    private void testTileVectorized(String tileId) throws IOException {
        var mvtFilePath = Paths.get(OMT_MVT_PATH, tileId + ".mvt" );

        var mvTile = MvtUtils.decodeMvt(mvtFilePath);
        var columnMapping = new ColumnMapping("name", ":", true);
        var columnMappings = Optional.of(List.of(columnMapping));
        var tileMetadata = MltConverter.createTilesetMetadata(mvTile, columnMappings, true);

        var allowIdRegeneration = true;
        var allowSorting = false;
        //var allowIdRegeneration = true;
        //var allowSorting = true;
        var optimization = new FeatureTableOptimizations(allowSorting, allowIdRegeneration, columnMappings);
        //TODO: fix -> either add columMappings per layer or global like when creating the scheme
        var optimizations = Map.of("place", optimization, "water_name", optimization, "transportation", optimization);
        var mlTile = MltConverter.convertMvt(mvTile, new ConversionConfig(true, false, optimizations),
                tileMetadata);

        var decodedTile = MltDecoder.decodeMlTileVectorized(mlTile, tileMetadata);

        compareTilesVectorized(decodedTile, mvTile);

        System.out.println("Ratio: " + Files.readAllBytes(mvtFilePath).length / (double)mlTile.length );
        System.out.println("Reduction: " + ((1 - (double)mlTile.length / Files.readAllBytes(mvtFilePath).length)*100));
    }

    public static void compareTilesVectorized(FeatureTable[] featureTables, MapboxVectorTile mvTile){
        var mvtLayers = mvTile.layers();

        for(var i = 0; i < mvtLayers.size(); i++){
            var featureTable = featureTables[i];
            var mvtLayer = mvtLayers.get(i);
            var mvtFeatures = mvtLayer.features();
            for(var j = 0; j < mvtFeatures.size(); j++){
                var mvtFeature = mvtFeatures.get(j);

                var mltFeature = featureTable.iterator().next();
                System.out.println(mltFeature);

                //var mltFeature = mltFeatures.stream().filter(f -> f.id() == mvtFeature.id()).findFirst().get();
                //var mvtId = mvtFeature.id();
                //var mltId = mltFeature.id();

                /*var mvtProperties = mvtFeature.properties();
                for(var mvtProperty : mvtProperties.entrySet()){
                }*/
            }
        }
    }
}
