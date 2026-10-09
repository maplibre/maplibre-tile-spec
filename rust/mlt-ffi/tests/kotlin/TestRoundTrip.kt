import org.maplibre.mlt_ffi.ConvertError
import org.maplibre.mlt_ffi.ConvertErrorKind
import org.maplibre.mlt_ffi.MltConverter
import org.maplibre.mlt_ffi.MltEncoderOptions
import org.maplibre.mlt_ffi.MltWireVersion
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

@OptIn(ExperimentalUnsignedTypes::class)
class TestRoundTrip {
    // test/fixtures/simple/point-boolean.mvt
    private val mvtFixture =
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

    @Test
    fun roundTrip() {
        val opts = MltEncoderOptions.new_()

        // MVT -> MLT
        val mltBuf = MltConverter.mvtToMlt(mvtFixture, opts).getOrThrow()
        val mltBytes = mltBuf.asBytes()
        assertTrue(mltBytes.isNotEmpty(), "MLT output is empty")

        // MLT -> MVT
        val mvtBuf = MltConverter.mltToMvt(mltBytes).getOrThrow()
        val mvtBytes = mvtBuf.asBytes()
        assertTrue(mvtBytes.isNotEmpty(), "MVT output is empty")

        assertEquals(0x1au.toUByte(), mvtBytes[0], "expected protobuf tag 0x1a")
    }

    @Test
    fun v2EncodingOfAPointLayerDecodesToTheSameMvtAsV1() {
        val v1Options = MltEncoderOptions.new_()
        val v2Options = MltEncoderOptions.new_()
        v2Options.setWireVersion(MltWireVersion.V02)

        val v1Mlt = MltConverter.mvtToMlt(mvtFixture, v1Options).getOrThrow().asBytes()
        val v2Mlt = MltConverter.mvtToMlt(mvtFixture, v2Options).getOrThrow().asBytes()

        assertContentEquals(
            MltConverter.mltToMvt(v1Mlt).getOrThrow().asBytes(),
            MltConverter.mltToMvt(v2Mlt).getOrThrow().asBytes(),
        )
    }

    @Test
    fun decodingGarbageAsMltReportsInvalidInputWithTheParserMessage() {
        val error = assertFailsWith<ConvertError> { MltConverter.mltToMvt(ubyteArrayOf(0xffu)).getOrThrow() }

        assertEquals(ConvertErrorKind.InvalidInput, error.kind())
        assertEquals("buffer underflow: needed 2 bytes, but only 1 remain", error.message())
    }

    @Test
    fun encodingGarbageAsMvtReportsEncodingFailedWithTheParserMessage() {
        val error =
            assertFailsWith<ConvertError> {
                MltConverter.mvtToMlt(ubyteArrayOf(0xffu), MltEncoderOptions.new_()).getOrThrow()
            }

        assertEquals(ConvertErrorKind.EncodingFailed, error.kind())
        assertEquals("MVT error: protobuf decode error: unexpected end of buffer", error.message())
    }
}
