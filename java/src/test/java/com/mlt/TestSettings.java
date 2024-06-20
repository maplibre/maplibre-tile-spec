package com.mlt;

import java.nio.file.Paths;
import java.util.List;

public class TestSettings {
  public static final String OMT_MVT_PATH = Paths.get("..", "test", "fixtures", "omt").toString();
  public static final String BING_MVT_PATH = Paths.get("..", "test", "fixtures", "bing").toString();
  public static final String AMZ_HERE_MVT_PATH =
      Paths.get("..", "test", "fixtures", "amazon_here").toString();
  public static final List<String> OPTIMIZED_MVT_LAYERS =
      List.of(
          "place",
          "water_name",
          "transportation",
          "transportation_name",
          "park",
          "mountain_peak",
          "poi",
          "waterway",
          "aerodrome_label",
          "water_feature",
          "island",
          "country_region",
          "populated_place",
          "admin_division1",
          "national_park",
          "housenumber",
          "continent",
          "sov_capital",
          "road");
  /* Layers with ids which are unique per tile but contain no global information, so they can be reassigned
   *  by the converter */
  public static final List<String> ID_REASSIGNABLE_MVT_LAYERS =
      List.of("transportation", "housenumber");
}
