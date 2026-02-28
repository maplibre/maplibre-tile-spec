package org.maplibre.mlt.converter;

import static org.junit.jupiter.api.Assertions.*;

import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import org.junit.jupiter.api.Test;

public class ConversionConfigTest {

  private static Map<String, FeatureTableOptimizations> sampleOptimizations() {
    var m = new HashMap<String, FeatureTableOptimizations>();
    m.put("layerA", new FeatureTableOptimizations(true, false, List.of()));
    return m;
  }

  private static void assertConfigEquals(ConversionConfig expected, ConversionConfig actual) {
    assertEquals(expected.getIncludeIds(), actual.getIncludeIds(), "includeIds");
    assertEquals(expected.getUseFastPFOR(), actual.getUseFastPFOR(), "useFastPFOR");
    assertEquals(expected.getUseFSST(), actual.getUseFSST(), "useFSST");
    assertEquals(
        expected.getTypeMismatchPolicy(), actual.getTypeMismatchPolicy(), "mismatchPolicy");
    assertEquals(expected.getOptimizations(), actual.getOptimizations(), "optimizations");
    assertEquals(
        expected.getUseMortonEncoding(), actual.getUseMortonEncoding(), "useMortonEncoding");
    assertEquals(
        expected.getPreTessellatePolygons(),
        actual.getPreTessellatePolygons(),
        "preTessellatePolygons");
    assertEquals(
        expected.getOutlineFeatureTableNames(), actual.getOutlineFeatureTableNames(), "outline");

    var expPattern = expected.getLayerFilterPattern();
    var actPattern = actual.getLayerFilterPattern();
    if (expPattern == null) {
      assertNull(actPattern, "layerFilterPattern");
    } else {
      assertNotNull(actPattern, "layerFilterPattern-not-null");
      assertEquals(expPattern.pattern(), actPattern.pattern(), "layerFilterPattern.value");
    }

    assertEquals(
        expected.getLayerFilterInvert(), actual.getLayerFilterInvert(), "layerFilterInvert");
    assertEquals(
        expected.getIntegerEncodingOption(), actual.getIntegerEncodingOption(), "integerEncoding");
    assertEquals(
        expected.getGeometryEncodingOption(),
        actual.getGeometryEncodingOption(),
        "geometryEncoding");
  }

  @Test
  public void testDefaults_fromNoArgConstructor() {
    var cfg = new ConversionConfig();

    assertEquals(ConversionConfig.DEFAULT_INCLUDE_IDS, cfg.getIncludeIds());
    assertEquals(ConversionConfig.DEFAULT_USE_FAST_PFOR, cfg.getUseFastPFOR());
    assertEquals(ConversionConfig.DEFAULT_USE_FSST, cfg.getUseFSST());
    assertEquals(ConversionConfig.DEFAULT_MISMATCH_POLICY, cfg.getTypeMismatchPolicy());
    assertNotNull(cfg.getOptimizations(), "optimizations-not-null");
    assertTrue(cfg.getOptimizations().isEmpty(), "optimizations-empty");
    assertEquals(ConversionConfig.DEFAULT_USE_MORTON_ENCODING, cfg.getUseMortonEncoding());
    assertEquals(ConversionConfig.DEFAULT_PRE_TESSELLATE_POLYGONS, cfg.getPreTessellatePolygons());
    assertNotNull(cfg.getOutlineFeatureTableNames(), "outline-not-null");
    assertTrue(cfg.getOutlineFeatureTableNames().isEmpty(), "outline-empty");
    assertNull(cfg.getLayerFilterPattern(), "layerFilterPattern-default-null");
    assertEquals(ConversionConfig.DEFAULT_LAYER_FILTER_INVERT, cfg.getLayerFilterInvert());
    assertEquals(ConversionConfig.DEFAULT_INTEGER_ENCODING, cfg.getIntegerEncodingOption());
    assertEquals(ConversionConfig.DEFAULT_INTEGER_ENCODING, cfg.getGeometryEncodingOption());
  }

  @Test
  public void testFullConstructor_nullOptimizationsAndOutlineBecomeEmpty() {
    var cfg =
        new ConversionConfig(
            /* includeIds= */ false,
            /* useFastPFOR= */ true,
            /* useFSST= */ true,
            /* mismatchPolicy= */ ConversionConfig.TypeMismatchPolicy.COERCE,
            /* optimizations= */ null,
            /* preTessellatePolygons= */ true,
            /* useMortonEncoding= */ false,
            /* outlineFeatureTableNames= */ null,
            /* layerFilterPattern= */ null,
            /* layerFilterInvert= */ true,
            /* integerEncodingOption= */ ConversionConfig.IntegerEncodingOption.DELTA,
            /* geometryEncodingOption= */ ConversionConfig.IntegerEncodingOption.DELTA);

    assertNotNull(cfg.getOptimizations());
    assertTrue(cfg.getOptimizations().isEmpty());
    assertNotNull(cfg.getOutlineFeatureTableNames());
    assertTrue(cfg.getOutlineFeatureTableNames().isEmpty());
  }

  @Test
  public void testConstructor_preservesPassedOptimizationsAndOutline() {
    var optim = sampleOptimizations();
    var outline = List.of("layerA");

    var cfg =
        new ConversionConfig(
            /* includeIds= */ true,
            /* useFastPFOR= */ false,
            /* useFSST= */ false,
            /* mismatchPolicy= */ ConversionConfig.TypeMismatchPolicy.ELIDE,
            /* optimizations= */ optim,
            /* preTessellatePolygons= */ false,
            /* useMortonEncoding= */ true,
            /* outlineFeatureTableNames= */ outline,
            /* layerFilterPattern= */ Pattern.compile("layerA"),
            /* layerFilterInvert= */ false,
            /* integerEncodingOption= */ ConversionConfig.IntegerEncodingOption.PLAIN,
            /* geometryEncodingOption= */ ConversionConfig.IntegerEncodingOption.PLAIN);

    assertSame(optim, cfg.getOptimizations(), "optimizations-same-reference");
    assertEquals(outline, cfg.getOutlineFeatureTableNames(), "outline-equals");
  }

  @Test
  public void testBuilder_setsAllFields_andBuildProducesEquivalentConfig() {
    var optim = sampleOptimizations();
    var outline = List.of("layerA");
    var pattern = Pattern.compile("^layer.*$");

    var built =
        ConversionConfig.builder()
            .includeIds(false)
            .useFastPFOR(true)
            .useFSST(true)
            .mismatchPolicy(ConversionConfig.TypeMismatchPolicy.COERCE)
            .optimizations(optim)
            .preTessellatePolygons(true)
            .useMortonEncoding(false)
            .outlineFeatureTableNames(outline)
            .layerFilterPattern(pattern)
            .layerFilterInvert(true)
            .integerEncoding(ConversionConfig.IntegerEncodingOption.DELTA)
            .build();

    assertEquals(false, built.getIncludeIds());
    assertEquals(true, built.getUseFastPFOR());
    assertEquals(true, built.getUseFSST());
    assertEquals(ConversionConfig.TypeMismatchPolicy.COERCE, built.getTypeMismatchPolicy());
    assertEquals(optim, built.getOptimizations());
    assertEquals(true, built.getPreTessellatePolygons());
    assertEquals(false, built.getUseMortonEncoding());
    assertEquals(outline, built.getOutlineFeatureTableNames());
    assertNotNull(built.getLayerFilterPattern());
    assertEquals(pattern.pattern(), built.getLayerFilterPattern().pattern());
    assertEquals(true, built.getLayerFilterInvert());
    assertEquals(ConversionConfig.IntegerEncodingOption.DELTA, built.getIntegerEncodingOption());
  }

  @Test
  public void testAsBuilder_roundTrip() {
    var orig =
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(true)
            .useFSST(false)
            .mismatchPolicy(ConversionConfig.TypeMismatchPolicy.ELIDE)
            .optimizations(sampleOptimizations())
            .preTessellatePolygons(false)
            .useMortonEncoding(true)
            .outlineFeatureTableNames(List.of("layerA"))
            .layerFilterPattern(Pattern.compile("layerA"))
            .layerFilterInvert(false)
            .integerEncoding(ConversionConfig.IntegerEncodingOption.PLAIN)
            .build();

    var rebuilt = orig.asBuilder().build();
    assertConfigEquals(orig, rebuilt);
  }

  @Test
  public void testMismatchPolicy_booleanOverload() {
    var coerce = ConversionConfig.builder().mismatchPolicy(true, false).build();
    assertEquals(ConversionConfig.TypeMismatchPolicy.COERCE, coerce.getTypeMismatchPolicy());

    var elide = ConversionConfig.builder().mismatchPolicy(false, true).build();
    assertEquals(ConversionConfig.TypeMismatchPolicy.ELIDE, elide.getTypeMismatchPolicy());

    var fail = ConversionConfig.builder().mismatchPolicy(false, false).build();
    assertEquals(ConversionConfig.TypeMismatchPolicy.FAIL, fail.getTypeMismatchPolicy());
  }

  @Test
  public void testBuilder_optimizations_null_isHandled() {
    var cfg = ConversionConfig.builder().optimizations(null).build();
    assertNotNull(cfg.getOptimizations());
    assertTrue(cfg.getOptimizations().isEmpty());
  }

  @Test
  public void testLayerFilterPatternAndInvert_behavior() {
    var pattern = Pattern.compile("layerA");
    var cfg =
        ConversionConfig.builder().layerFilterPattern(pattern).layerFilterInvert(true).build();
    assertNotNull(cfg.getLayerFilterPattern());
    assertEquals("layerA", cfg.getLayerFilterPattern().pattern());
    assertTrue(cfg.getLayerFilterInvert());

    var cfg2 = ConversionConfig.builder().layerFilterPattern(null).layerFilterInvert(false).build();
    assertNull(cfg2.getLayerFilterPattern());
    assertFalse(cfg2.getLayerFilterInvert());
  }

  @Test
  public void testIntegerEncodingOption_defaultsAndCustom() {
    var defaultCfg = new ConversionConfig();
    assertEquals(ConversionConfig.DEFAULT_INTEGER_ENCODING, defaultCfg.getIntegerEncodingOption());

    var custom =
        ConversionConfig.builder()
            .integerEncoding(ConversionConfig.IntegerEncodingOption.RLE)
            .build();
    assertEquals(ConversionConfig.IntegerEncodingOption.RLE, custom.getIntegerEncodingOption());
  }

  @Test
  public void testGeometryEncodingOption_defaultsAndCustom() {
    var defaultCfg = new ConversionConfig();
    assertEquals(ConversionConfig.DEFAULT_INTEGER_ENCODING, defaultCfg.getGeometryEncodingOption());

    var custom =
        ConversionConfig.builder()
            .geometryEncoding(ConversionConfig.IntegerEncodingOption.PLAIN)
            .build();
    assertEquals(ConversionConfig.IntegerEncodingOption.PLAIN, custom.getGeometryEncodingOption());

    // When only integer encoding is set, geometry stays AUTO (backward compatible with main:
    // on main, geometry streams always used AUTO and were not controlled by integerEncodingOption).
    var withIntegerOnly =
        ConversionConfig.builder()
            .integerEncoding(ConversionConfig.IntegerEncodingOption.RLE)
            .build();
    assertEquals(
        ConversionConfig.IntegerEncodingOption.RLE, withIntegerOnly.getIntegerEncodingOption());
    assertEquals(
        ConversionConfig.DEFAULT_INTEGER_ENCODING, withIntegerOnly.getGeometryEncodingOption());
  }

  @Test
  public void testIntegerEncodingOnlyConstructor_geometryStaysAuto() {
    // Constructor that takes integerEncodingOption: geometry encoding stays AUTO for backward
    // compatibility (on main, geometry was never configurable and always used AUTO).
    var cfg =
        new ConversionConfig(
            true, false, false, Map.of(), false, ConversionConfig.IntegerEncodingOption.DELTA);
    assertEquals(ConversionConfig.IntegerEncodingOption.DELTA, cfg.getIntegerEncodingOption());
    assertEquals(ConversionConfig.DEFAULT_INTEGER_ENCODING, cfg.getGeometryEncodingOption());
  }
}
