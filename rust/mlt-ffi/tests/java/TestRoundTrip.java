// Round-trip test: MVT -> MLT -> MVT via the Java Panama FFM bindings.
import org.maplibre.mlt.MltConverter;
import org.maplibre.mlt.MltEncoderOptions;

public class TestRoundTrip {

    // test/fixtures/simple/point-boolean.mvt (35 bytes)
    static final byte[] MVT_FIXTURE = {
        0x1a, 0x21, 0x0a, 0x05, 0x6c, 0x61, 0x79, 0x65, 0x72, 0x12, 0x0d, 0x08,
        0x01, 0x12, 0x02, 0x00, 0x00, 0x18, 0x01, 0x22, 0x03, 0x09, 0x32, 0x22,
        0x1a, 0x03, 0x6b, 0x65, 0x79, 0x22, 0x02, 0x38, 0x01, 0x78, 0x02
    };

    public static void main(String[] args) throws Exception {
        // 1. Create default encoder options
        try (var opts = new MltEncoderOptions()) {
            // 2. MVT -> MLT
            byte[] mltBytes = MltConverter.mvtToMlt(MVT_FIXTURE, opts);
            assert mltBytes.length > 0 : "MLT output is empty";

            // 3. MLT -> MVT
            byte[] mvtBytes = MltConverter.mltToMvt(mltBytes);
            assert mvtBytes.length > 0 : "MVT output is empty";

            // 4. Verify round-trip produced valid MVT
            assert mvtBytes[0] == 0x1a : "expected protobuf tag 0x1a, got " + mvtBytes[0];
        }

        System.out.println("Java round-trip test passed");
    }
}
