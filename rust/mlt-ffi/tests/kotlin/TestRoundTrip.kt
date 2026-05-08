// Round-trip test: MVT -> MLT -> MVT via the Kotlin diplomat bindings.
import org.maplibre.mlt_ffi.MltConverter
import org.maplibre.mlt_ffi.MltEncoderOptions


// test/fixtures/simple/point-boolean.mvt (35 bytes)
@OptIn(ExperimentalUnsignedTypes::class)
val MVT_FIXTURE =
    ubyteArrayOf(
        0x1au,
        0x21u,
        0x0au,
        0x05u,
        0x6cu,
        0x61u,
        0x79u,
        0x65u,
        0x72u,
        0x12u,
        0x0du,
        0x08u,
        0x01u,
        0x12u,
        0x02u,
        0x00u,
        0x00u,
        0x18u,
        0x01u,
        0x22u,
        0x03u,
        0x09u,
        0x32u,
        0x22u,
        0x1au,
        0x03u,
        0x6bu,
        0x65u,
        0x79u,
        0x22u,
        0x02u,
        0x38u,
        0x01u,
        0x78u,
        0x02u,
    )

@OptIn(ExperimentalUnsignedTypes::class)
fun main() {
    // 1. Create default encoder options
    val opts = MltEncoderOptions.new_()

    // 2. MVT -> MLT
    val mltBuf = MltConverter.mvtToMlt(MVT_FIXTURE, opts).getOrThrow()
    val mltBytes = mltBuf.asBytes()
    check(mltBytes.isNotEmpty()) { "MLT output is empty" }

    // 3. MLT -> MVT
    val mvtBuf = MltConverter.mltToMvt(mltBytes).getOrThrow()
    val mvtBytes = mvtBuf.asBytes()
    check(mvtBytes.isNotEmpty()) { "MVT output is empty" }

    // 4. Verify round-trip produced valid MVT
    check(mvtBytes[0] == 0x1au.toUByte()) { "expected protobuf tag 0x1a, got ${mvtBytes[0]}" }

    println("Kotlin round-trip test passed")
}
