package org.maplibre.mlt.converter;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertTrue;

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
    assertEquals(expected.includeIds(), actual.includeIds(), "includeIds");
    assertEquals(expected.useFastPFOR(), actual.useFastPFOR(), "useFastPFOR");
    assertEquals(expected.useFSST(), actual.useFSST(), "useFSST");
    assertEquals(expected.typeMismatchPolicy(), actual.typeMismatchPolicy(), "typeMismatchPolicy");
    assertEquals(expected.optimizations(), actual.optimizations(), "optimizations");
    assertEquals(expected.useMortonEncoding(), actual.useMortonEncoding(), "useMortonEncoding");
    assertEquals(
        expected.preTessellatePolygons(), actual.preTessellatePolygons(), "preTessellatePolygons");
    assertEquals(expected.outlineFeatureTableNames(), actual.outlineFeatureTableNames(), "outline");

    var expPattern = expected.layerFilterPattern();
    var actPattern = actual.layerFilterPattern();
    if (expPattern == null) {
      assertNull(actPattern, "layerFilterPattern");
    } else {
      assertNotNull(actPattern, "layerFilterPattern-not-null");
      assertEquals(expPattern.pattern(), actPattern.pattern(), "layerFilterPattern.value");
    }

    assertEquals(expected.layerFilterInvert(), actual.layerFilterInvert(), "layerFilterInvert");
    assertEquals(
        expected.integerEncodingOption(), actual.integerEncodingOption(), "integerEncoding");
    assertEquals(
        expected.geometryEncodingOption(), actual.geometryEncodingOption(), "geometryEncoding");
  }

  @Test
  public void testDefaults_fromNoArgConstructor() {
    var cfg = ConversionConfig.builder().build();

    assertEquals(ConversionConfig.DEFAULT_INCLUDE_IDS, cfg.includeIds());
    assertEquals(ConversionConfig.DEFAULT_USE_FAST_PFOR, cfg.useFastPFOR());
    assertEquals(ConversionConfig.DEFAULT_USE_FSST, cfg.useFSST());
    assertEquals(ConversionConfig.DEFAULT_MISMATCH_POLICY, cfg.typeMismatchPolicy());
    assertNotNull(cfg.optimizations(), "optimizations-not-null");
    assertTrue(cfg.optimizations().isEmpty(), "optimizations-empty");
    assertEquals(ConversionConfig.DEFAULT_USE_MORTON_ENCODING, cfg.useMortonEncoding());
    assertEquals(ConversionConfig.DEFAULT_PRE_TESSELLATE_POLYGONS, cfg.preTessellatePolygons());
    assertNotNull(cfg.outlineFeatureTableNames(), "outline-not-null");
    assertTrue(cfg.outlineFeatureTableNames().isEmpty(), "outline-empty");
    assertNull(cfg.layerFilterPattern(), "layerFilterPattern-default-null");
    assertEquals(ConversionConfig.DEFAULT_LAYER_FILTER_INVERT, cfg.layerFilterInvert());
    assertEquals(ConversionConfig.DEFAULT_INTEGER_ENCODING, cfg.integerEncodingOption());
    assertEquals(ConversionConfig.DEFAULT_INTEGER_ENCODING, cfg.geometryEncodingOption());
  }

  @Test
  public void testFullConstructor_nullOptimizationsAndOutlineBecomeEmpty() {
    var cfg =
        ConversionConfig.builder()
            .includeIds(false)
            .useFastPFOR(true)
            .useFSST(true)
            .typeMismatchPolicy(ConversionConfig.TypeMismatchPolicy.COERCE)
            .optimizations(null)
            .preTessellatePolygons(true)
            .useMortonEncoding(false)
            .outlineFeatureTableNames(null)
            .layerFilterPattern(null)
            .layerFilterInvert(true)
            .integerEncoding(ConversionConfig.IntegerEncodingOption.DELTA)
            .geometryEncoding(ConversionConfig.IntegerEncodingOption.DELTA)
            .build();

    assertNotNull(cfg.optimizations());
    assertTrue(cfg.optimizations().isEmpty());
    assertNotNull(cfg.outlineFeatureTableNames());
    assertTrue(cfg.outlineFeatureTableNames().isEmpty());
  }

  @Test
  public void testConstructor_preservesPassedOptimizationsAndOutline() {
    var optim = sampleOptimizations();
    var outline = List.of("layerA");

    var cfg =
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(false)
            .useFSST(false)
            .typeMismatchPolicy(ConversionConfig.TypeMismatchPolicy.ELIDE)
            .optimizations(optim)
            .preTessellatePolygons(false)
            .useMortonEncoding(true)
            .outlineFeatureTableNames(outline)
            .layerFilterPattern(Pattern.compile("layerA"))
            .layerFilterInvert(false)
            .integerEncoding(ConversionConfig.IntegerEncodingOption.PLAIN)
            .geometryEncoding(ConversionConfig.IntegerEncodingOption.PLAIN)
            .build();

    assertSame(optim, cfg.optimizations(), "optimizations-same-reference");
    assertEquals(outline, cfg.outlineFeatureTableNames(), "outline-equals");
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
            .typeMismatchPolicy(ConversionConfig.TypeMismatchPolicy.COERCE)
            .optimizations(optim)
            .preTessellatePolygons(true)
            .useMortonEncoding(false)
            .outlineFeatureTableNames(outline)
            .layerFilterPattern(pattern)
            .layerFilterInvert(true)
            .integerEncoding(ConversionConfig.IntegerEncodingOption.DELTA)
            .build();

    assertEquals(false, built.includeIds());
    assertEquals(true, built.useFastPFOR());
    assertEquals(true, built.useFSST());
    assertEquals(ConversionConfig.TypeMismatchPolicy.COERCE, built.typeMismatchPolicy());
    assertEquals(optim, built.optimizations());
    assertEquals(true, built.preTessellatePolygons());
    assertEquals(false, built.useMortonEncoding());
    assertEquals(outline, built.outlineFeatureTableNames());
    assertNotNull(built.layerFilterPattern());
    assertEquals(pattern.pattern(), built.layerFilterPattern().pattern());
    assertEquals(true, built.layerFilterInvert());
    assertEquals(ConversionConfig.IntegerEncodingOption.DELTA, built.integerEncodingOption());
  }

  @Test
  public void testAsBuilder_roundTrip() {
    var orig =
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(true)
            .useFSST(false)
            .typeMismatchPolicy(ConversionConfig.TypeMismatchPolicy.ELIDE)
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
    var coerce = ConversionConfig.builder().typeMismatchPolicy(true, false).build();
    assertEquals(ConversionConfig.TypeMismatchPolicy.COERCE, coerce.typeMismatchPolicy());

    var elide = ConversionConfig.builder().typeMismatchPolicy(false, true).build();
    assertEquals(ConversionConfig.TypeMismatchPolicy.ELIDE, elide.typeMismatchPolicy());

    var fail = ConversionConfig.builder().typeMismatchPolicy(false, false).build();
    assertEquals(ConversionConfig.TypeMismatchPolicy.FAIL, fail.typeMismatchPolicy());
  }

  @Test
  public void testBuilder_optimizations_null_isHandled() {
    var cfg = ConversionConfig.builder().optimizations(null).build();
    assertNotNull(cfg.optimizations());
    assertTrue(cfg.optimizations().isEmpty());
  }

  @Test
  public void testLayerFilterPatternAndInvert_behavior() {
    var pattern = Pattern.compile("layerA");
    var cfg =
        ConversionConfig.builder().layerFilterPattern(pattern).layerFilterInvert(true).build();
    assertNotNull(cfg.layerFilterPattern());
    assertEquals("layerA", cfg.layerFilterPattern().pattern());
    assertTrue(cfg.layerFilterInvert());

    var cfg2 = ConversionConfig.builder().layerFilterPattern(null).layerFilterInvert(false).build();
    assertNull(cfg2.layerFilterPattern());
    assertFalse(cfg2.layerFilterInvert());
  }

  @Test
  public void testIntegerEncodingOption_defaultsAndCustom() {
    var defaultCfg = ConversionConfig.builder().build();
    assertEquals(ConversionConfig.DEFAULT_INTEGER_ENCODING, defaultCfg.integerEncodingOption());

    var custom =
        ConversionConfig.builder()
            .integerEncoding(ConversionConfig.IntegerEncodingOption.RLE)
            .build();
    assertEquals(ConversionConfig.IntegerEncodingOption.RLE, custom.integerEncodingOption());
  }

  @Test
  public void testGeometryEncodingOption_defaultsAndCustom() {
    var defaultCfg = ConversionConfig.builder().build();
    assertEquals(ConversionConfig.DEFAULT_INTEGER_ENCODING, defaultCfg.geometryEncodingOption());

    var custom =
        ConversionConfig.builder()
            .geometryEncoding(ConversionConfig.IntegerEncodingOption.PLAIN)
            .build();
    assertEquals(ConversionConfig.IntegerEncodingOption.PLAIN, custom.geometryEncodingOption());

    // When only integer encoding is set, geometry stays AUTO (backward compatible with main:
    // on main, geometry streams always used AUTO and were not controlled by integerEncodingOption).
    var withIntegerOnly =
        ConversionConfig.builder()
            .integerEncoding(ConversionConfig.IntegerEncodingOption.RLE)
            .build();
    assertEquals(
        ConversionConfig.IntegerEncodingOption.RLE, withIntegerOnly.integerEncodingOption());
    assertEquals(
        ConversionConfig.DEFAULT_INTEGER_ENCODING, withIntegerOnly.geometryEncodingOption());
  }

  @Test
  public void testIntegerEncodingOnlyConstructor_geometryStaysAuto() {
    // Constructor that takes integerEncodingOption: geometry encoding stays AUTO for backward
    // compatibility (on main, geometry was never configurable and always used AUTO).
    var cfg =
        ConversionConfig.builder()
            .includeIds(true)
            .useFastPFOR(false)
            .useFSST(false)
            .optimizations(Map.of())
            .preTessellatePolygons(false)
            .integerEncoding(ConversionConfig.IntegerEncodingOption.DELTA)
            .build();
    assertEquals(ConversionConfig.IntegerEncodingOption.DELTA, cfg.integerEncodingOption());
    assertEquals(ConversionConfig.DEFAULT_INTEGER_ENCODING, cfg.geometryEncodingOption());
  }
}
