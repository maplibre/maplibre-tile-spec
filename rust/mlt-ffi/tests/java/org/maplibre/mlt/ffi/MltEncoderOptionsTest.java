package org.maplibre.mlt.ffi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;

import org.junit.jupiter.api.Test;

class MltEncoderOptionsTest {

  @Test
  void optionsBuiltWithTheSameSettingsAreEqual() {
    var first = MltEncoderOptions.builder().wireVersion(MltWireVersion.V2).tessellate(true).build();
    var second = MltEncoderOptions.builder().tessellate(true).wireVersion(MltWireVersion.V2).build();

    assertEquals(first, second);
    assertEquals(first.hashCode(), second.hashCode());
  }

  @Test
  void optionsWithAnotherWireVersionDiffer() {
    var v1 = MltEncoderOptions.builder().wireVersion(MltWireVersion.V1).build();
    var v2 = MltEncoderOptions.builder().wireVersion(MltWireVersion.V2).build();

    assertNotEquals(v1, v2);
  }

  @Test
  void defaultsPrintNoSettings() {
    assertEquals("MltEncoderOptions{}", MltEncoderOptions.defaults().toString());
  }

  @Test
  void optionsPrintTheSettingsThatWereSet() {
    var options = MltEncoderOptions.builder()
      .allowFsst(false)
      .wireVersion(MltWireVersion.V2)
      .tessellate(true)
      .build();

    assertEquals("MltEncoderOptions{wireVersion=V2, TESSELLATE=true, FSST=false}", options.toString());
  }
}
