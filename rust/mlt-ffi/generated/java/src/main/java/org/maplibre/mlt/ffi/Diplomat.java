package org.maplibre.mlt.ffi;

import static java.lang.foreign.MemoryLayout.PathElement.groupElement;
import static java.lang.foreign.ValueLayout.JAVA_BYTE;

import java.lang.foreign.GroupLayout;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SegmentAllocator;
import java.util.List;
import org.maplibre.mlt.ffi.raw.DiplomatU8View;
import org.maplibre.mlt.ffi.raw.MltConverter_mlt_to_mvt_result;
import org.maplibre.mlt.ffi.raw.MltConverter_mlt_to_mvt_with_limit_result;
import org.maplibre.mlt.ffi.raw.MltConverter_mvt_to_mlt_result;
import org.maplibre.mlt.ffi.raw.MltLayerBuilder_begin_mvt_feature_result;
import org.maplibre.mlt.ffi.raw.MltLayerBuilder_encode_into_result;
import org.maplibre.mlt.ffi.raw.MltLayerBuilder_new_result;
import org.maplibre.mlt.ffi.raw.MltLayerBuilder_reset_result;
import org.maplibre.mlt.ffi.raw.MltLayerBuilder_set_bool_result;
import org.maplibre.mlt.ffi.raw.MltLayerBuilder_set_f32_result;
import org.maplibre.mlt.ffi.raw.MltLayerBuilder_set_f64_result;
import org.maplibre.mlt.ffi.raw.MltLayerBuilder_set_i64_result;
import org.maplibre.mlt.ffi.raw.MltLayerBuilder_set_str_result;

/**
 * Reads and writes the structs Diplomat passes across the native boundary.
 * <p>
 * Every result struct is a union of an ok and an error pointer followed by an {@code is_ok} flag.
 * The structs differ only in their names, so they are all read through {@link MltLayerBuilder_new_result}.
 */
final class Diplomat {

  /** The byte size of every result struct. */
  static final long RESULT_SIZE = MltLayerBuilder_new_result.sizeof();

  static {
    List<GroupLayout> results = List.of(
      MltConverter_mlt_to_mvt_result.layout(),
      MltConverter_mlt_to_mvt_with_limit_result.layout(),
      MltConverter_mvt_to_mlt_result.layout(),
      MltLayerBuilder_begin_mvt_feature_result.layout(),
      MltLayerBuilder_encode_into_result.layout(),
      MltLayerBuilder_reset_result.layout(),
      MltLayerBuilder_set_bool_result.layout(),
      MltLayerBuilder_set_f32_result.layout(),
      MltLayerBuilder_set_f64_result.layout(),
      MltLayerBuilder_set_i64_result.layout(),
      MltLayerBuilder_set_str_result.layout());
    long isOk = MltLayerBuilder_new_result.is_ok$offset();
    for (GroupLayout result : results) {
      long offset = result.byteOffset(groupElement("is_ok"));
      if (result.byteSize() != RESULT_SIZE || offset != isOk) {
        throw new IllegalStateException("Unexpected native result layout " + result);
      }
    }
  }

  private Diplomat() {}

  /**
   * Returns the ok pointer of a result.
   *
   * @throws MltException if the result holds an error
   */
  static MemorySegment unwrap(MemorySegment result) {
    check(result);
    return MltLayerBuilder_new_result.ok(result);
  }

  /**
   * Checks a result that carries no value.
   *
   * @throws MltException if the result holds an error
   */
  static void check(MemorySegment result) {
    if (!MltLayerBuilder_new_result.is_ok(result)) {
      throw MltException.fromNative(MltLayerBuilder_new_result.err(result));
    }
  }

  /** Copies {@code bytes} to native memory and returns a view of them. */
  static MemorySegment u8View(SegmentAllocator allocator, byte[] bytes) {
    MemorySegment view = DiplomatU8View.allocate(allocator);
    DiplomatU8View.data(view, allocator.allocateFrom(JAVA_BYTE, bytes));
    DiplomatU8View.len(view, bytes.length);
    return view;
  }

  /** Copies the bytes a view points at to the heap. */
  static byte[] toByteArray(MemorySegment u8View) {
    return DiplomatU8View.data(u8View).reinterpret(DiplomatU8View.len(u8View)).toArray(JAVA_BYTE);
  }
}
