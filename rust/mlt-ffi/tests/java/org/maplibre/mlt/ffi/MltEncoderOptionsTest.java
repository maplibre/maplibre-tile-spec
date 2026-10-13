package org.maplibre.mlt.ffi;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;

import org.junit.jupiter.api.Test;

class MltEncoderOptionsTest {

  @Test
  void optionsBuiltWithTheSameSettingsAreEqual() {
    var first = MltEncoder.builder().wireVersion(MltWireVersion.V2).tessellate(true).options();
    var second = MltEncoder.builder().tessellate(true).wireVersion(MltWireVersion.V2).options();

    assertEquals(first, second);
    assertEquals(first.hashCode(), second.hashCode());
  }

  @Test
  void optionsWithAnotherWireVersionDiffer() {
    var v1 = MltEncoder.builder().wireVersion(MltWireVersion.V1).options();
    var v2 = MltEncoder.builder().wireVersion(MltWireVersion.V2).options();

    assertNotEquals(v1, v2);
  }

  @Test
  void defaultsPrintNoSettings() {
    assertEquals("MltEncoderOptions{}", MltEncoder.builder().options().toString());
  }

  @Test
  void optionsPrintTheSettingsThatWereSet() {
    var options = MltEncoder.builder()
      .allowFsst(false)
      .wireVersion(MltWireVersion.V2)
      .tessellate(true)
      .options();

    assertEquals("MltEncoderOptions{wireVersion=V2, TESSELLATE=true, FSST=false}", options.toString());
  }

  @Test
  void defaultBuilderBuildsAnEncoderThatEncodesLikeTheNoArgumentOne() {
    try (var built = MltEncoder.builder().build(); var plain = new MltEncoder()) {
      for (var encoder : new MltEncoder[] {built, plain}) {
        encoder.beginLayer("layer", 4096);
        encoder.beginFeature(MvtGeometryType.POINT, new int[] {9, 2, 2});
      }

      assertArrayEquals(plain.toByteArray(), built.toByteArray());
    }
  }
}
