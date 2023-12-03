package com.covt.decoder;

import com.covt.converter.CovtConverter;
import com.covt.converter.mvt.Layer;
import org.junit.jupiter.api.Test;
import java.io.IOException;
import java.nio.file.Paths;
import java.sql.SQLException;
import java.util.List;
import java.util.Optional;

import static org.junit.jupiter.api.Assertions.assertEquals;

public class CovtParserTest {
    private static final String BING_MVT_PATH = ".\\data\\bing\\mvt";

    /* Bing tiles --------------------------------  */

    @Test
    public void parseBingCovt_Z4Tile_ValidParsedTile() throws IOException{
        var tileIds = List.of("4-8-5", "4-12-6", "4-13-6", "4-9-5");
        runBingTests(tileIds);
    }

    @Test
    public void parseBingCovt_Z5Tile_ValidParsedTile() throws IOException{
        var tileIds = List.of("5-16-11", "5-17-11", "5-17-10", "5-25-13", "5-26-13", "5-8-12", "5-15-10");
        runBingTests(tileIds);
    }

    @Test
    public void parseBingCovt_Z6Tile_ValidParsedTile() throws IOException{
        var tileIds = List.of("6-32-22", "6-33-22", "6-32-23", "6-32-21");
        runBingTests(tileIds);
    }

    @Test
    public void parseBingCovt_Z7Tile_ValidParsedTile() throws IOException{
        var tileIds = List.of("7-65-43", "7-64-44", "7-66-43", "7-66-44", "7-65-42", "7-66-42", "7-67-43");
        runBingTests(tileIds);
    }

    private void runBingTests(List<String> tileIds) throws IOException {
        for(var tileId : tileIds){
            var mvtTile = com.covt.converter.mvt.MvtUtils.decodeMvt2(Paths.get(BING_MVT_PATH, tileId + ".mvt"));
            var mvtLayers = mvtTile.layers();

            var covtTile = CovtConverter.convertMvtTile(mvtLayers, mvtTile.tileExtent(), CovtConverter.GeometryEncoding.ICE_MORTON,
                    true, true, false, false);

            var covtLayers = CovtParser.decodeCovt(covtTile);

            compareTiles(mvtLayers, covtLayers);
        }
    }

    private void compareTiles(List<Layer> mvtLayers, List<Layer> covtLayers){
        assertEquals(mvtLayers.size(), covtLayers.size());
        for(var i = 0; i < mvtLayers.size(); i++){
            var mvtLayer = mvtLayers.get(i);
            var covtLayer = covtLayers.get(i);

            assertEquals(mvtLayer.name(), covtLayer.name());

            var mvtFeatures = mvtLayer.features();
            var covtFeatures = covtLayer.features();
            for(var j = 0; j < mvtFeatures.size(); j++){
                var mvtFeature = mvtFeatures.get(j);
                var covtFeature = covtFeatures.get(j);

                assertEquals(mvtFeature.id(), covtFeature.id());
                assertEquals(mvtFeature.geometry(), covtFeature.geometry());

                var mvtProperties = mvtFeature.properties();
                var covtProperties = covtFeature.properties();
                for(var propertyKey : mvtProperties.keySet()){
                    var mvtProperty = mvtProperties.get(propertyKey);
                    var optionalCovtProperty = ((Optional)covtProperties.get(propertyKey));
                    var covtProperty = optionalCovtProperty.isPresent() ? optionalCovtProperty.get() : null;

                    assertEquals(mvtProperty, covtProperty);
                }
            }
        }
    }

}
