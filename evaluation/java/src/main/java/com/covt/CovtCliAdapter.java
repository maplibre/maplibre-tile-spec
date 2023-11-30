package com.covt;

import com.covt.converter.CovtConverter;
import com.covt.converter.mvt.MvtUtils;
import org.apache.commons.cli.*;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.SQLException;

public class CovtCliAdapter {
    private static final String FILE_NAME_ARG = "filename";
    private static final String ZOOM_LEVEL_ARG = "z";
    private static final String X_COORDINATE_ARG = "x";
    private static final String Y_COORDINATE_ARG = "y";
    private static final int NUM_COORDINATES_PER_QUADRANT = 8192;

    public static void main(String... args) throws ParseException, IOException, SQLException, ClassNotFoundException {
        Options options = new Options();
        options.addOption(FILE_NAME_ARG, true, "Name and path of the MBTiles archive");
        options.addOption(ZOOM_LEVEL_ARG, true, "Zoom level of the specific tile");
        options.addOption(X_COORDINATE_ARG, true, "X coordinate of the specific tile");
        options.addOption(Y_COORDINATE_ARG, true,"Y coordinate of the specific tile");
        CommandLineParser parser = new DefaultParser();
        var commandLine = parser.parse(options, args);

        var fileName = commandLine.getOptionValue(FILE_NAME_ARG);
        var z = Integer.parseInt(commandLine.getOptionValue(ZOOM_LEVEL_ARG));
        var x = Integer.parseInt(commandLine.getOptionValue(X_COORDINATE_ARG));
        var y = Integer.parseInt(commandLine.getOptionValue(Y_COORDINATE_ARG));

        var mvtTile = MvtUtils.decodeMvt(fileName, z, x, y);
        var mvtLayers = mvtTile.layers();

        var tile = CovtConverter.convertMvtTile(mvtLayers, NUM_COORDINATES_PER_QUADRANT,
                CovtConverter.GeometryEncoding.ICE, true, true, true);

        var covtFileName = String.format("%s_%s_%s.covt", z, x, y);
        Files.write(Path.of(covtFileName), tile);
    }
}
