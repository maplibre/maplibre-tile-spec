just := quote(just_executable())

_default: (just '--list' 'cpp')

[private]
just *args:
    {{just}} {{args}}

# Initialize CMake build
cmake-init:
    cmake -B build -S . -DCMAKE_BUILD_TYPE=Coverage

# Build with CMake
cmake-build: cmake-init
    cmake --build build --target mlt-cpp-test mlt-cpp-json

# Quick compile check (CMake build)
check: cmake-build

# Run all CI steps: test, coverage, bazel build
ci-test: coverage bazel-build
    {{just}} assert-git-is-clean

# Delete build artifacts
clean:
    rm -rf build

# Reformat code
fmt:
    echo "TODO: Add C++ formatting command"

# Run linting
lint:
    echo "TODO: Add C++ linting command"

# Run tests
[working-directory: 'build']
test: cmake-build
    ctest
    {{just}} cpp::decode-synthetics

# Generate coverage report
[working-directory: 'build']
coverage: (_check-tool 'gcovr') test
    gcovr --root ../.. \
        --filter ../src --filter ../include \
        --txt coverage.txt \
        --merge-mode-functions=separate \
        --cobertura-pretty --cobertura coverage.xml \
        --html-details coverage.html
    @echo "Coverage report at $PWD/coverage.html"

# Build with Bazel
bazel-build:
    bazel build //cpp:mlt_cpp

# Sanity test for mlt-cpp-json tool
test-json:
    build/tool/mlt-cpp-json ../test/expected/tag0x01/bing/4-12-6.mlt | jq .

decode-synthetics: cmake-build
    #!/usr/bin/env bash -e
    # These are the ones that work currently
    # Ideally just: `fd . test/synthetic/0x01 -e mlt -x cpp/build/tool/mlt-cpp-json "{}" >/dev/null`
    JSON_TOOL=build/tool/mlt-cpp-json
    TEST_BASE=../test/synthetic/0x01
    MLTS=(
        prop_u64_min.mlt
        mix_2_line_line.mlt
        mix_2_mpt_mpt.mlt
        props_u32_rle.mlt
        prop_bool_false_null.mlt
        mix_3_mline_poly_mline.mlt
        mix_3_pt_mpt_pt.mlt
        ids64.mlt
        prop_f64_null_val.mlt
        prop_u32_val_null.mlt
        mix_3_pt_line_mpoly.mlt
        prop_f64_pos_inf.mlt
        extent_buf_1073741824.mlt
        mix_3_pt_polyh_mpoly.mlt
        prop_i64_null_val.mlt
        props_no_shared_dict.mlt
        prop_f32_min_val.mlt
        mix_3_mpt_pt_mpt.mlt
        mix_3_polyh_mpoly_polyh.mlt
        mix_4_pt_line_polyh_mline.mlt
        multipoint.mlt
        extent_buf_4096.mlt
        mix_3_poly_polyh_poly.mlt
        prop_f64_neg_inf.mlt
        mix_5_pt_line_polyh_mline_mpoly.mlt
        mix_4_pt_line_poly_mpoly.mlt
        mix_2_poly_mpoly.mlt
        mix_3_mline_polyh_mline.mlt
        prop_i64_neg.mlt
        mix_3_pt_line_poly.mlt
        extent_131072.mlt
        prop_f64_val_null.mlt
        ids64_delta.mlt
        mix_3_line_poly_mline.mlt
        ids64_delta_rle.mlt
        prop_bool.mlt
        mix_3_line_polyh_mpoly.mlt
        prop_u32_null_val.mlt
        extent_512.mlt
        mix_3_poly_mline_mpoly.mlt
        id0.mlt
        mix_2_line_mline.mlt
        mix_2_pt_poly.mlt
        line.mlt
        mix_3_pt_line_pt.mlt
        props_u32_delta.mlt
        props_shared_dict.mlt
        props_u64_delta.mlt
        prop_u32.mlt
        prop_f32_zero.mlt
        extent_buf_512.mlt
        props_i32.mlt
        ids.mlt
        prop_i64_val_null.mlt
        mix_5_pt_line_poly_polyh_mline.mlt
        prop_i64_max.mlt
        mix_3_pt_poly_mline.mlt
        prop_str_empty.mlt
        mix_2_polyh_mline.mlt
        props_u32.mlt
        mix_4_pt_poly_mline_mpoly.mlt
        prop_i32.mlt
        extent_1073741824.mlt
        prop_i32_max.mlt
        mix_2_pt_polyh.mlt
        prop_bool_true_null.mlt
        mix_4_pt_line_poly_mline.mlt
        mix_2_poly_mline.mlt
        props_mixed.mlt
        mix_4_pt_line_polyh_mpoly.mlt
        polygon_tes.mlt
        props_u64_delta_rle.mlt
        mix_3_poly_mline_poly.mlt
        mix_2_line_poly.mlt
        mix_5_pt_poly_polyh_mline_mpoly.mlt
        mix_3_pt_polyh_mline.mlt
        mix_3_pt_line_mline.mlt
        props_i32_delta.mlt
        mix_3_pt_polyh_pt.mlt
        mix_3_pt_mline_pt.mlt
        ids64_rle.mlt
        prop_i32_neg.mlt
        mix_4_line_poly_mline_mpoly.mlt
        id.mlt
        mix_5_line_poly_polyh_mline_mpoly.mlt
        prop_bool_null_true.mlt
        mix_3_mline_line_mline.mlt
        mix_2_polyh_mpoly.mlt
        mix_3_pt_poly_mpoly.mlt
        mix_2_pt_mpt.mlt
        prop_f32_neg_zero.mlt
        mix_5_pt_line_poly_polyh_mpoly.mlt
        mix_2_line_mpt.mlt
        mix_3_poly_pt_poly.mlt
        mix_3_line_polyh_mline.mlt
        mix_3_line_mpoly_line.mlt
        prop_str_escape.mlt
        mix_3_line_poly_mpoly.mlt
        prop_f64_min_norm.mlt
        ids_delta_rle.mlt
        prop_f64.mlt
        mix_3_polyh_mline_polyh.mlt
        mix_3_mpoly_mline_mpoly.mlt
        ids_rle.mlt
        props_u64_rle.mlt
        mix_2_pt_line.mlt
        prop_u32_min.mlt
        props_str_fsst.mlt
        mix_2_line_mpoly.mlt
        mix_4_line_poly_polyh_mpoly.mlt
        prop_u64_max.mlt
        mix_3_pt_mpoly_pt.mlt
        prop_str_null_val.mlt
        mix_3_pt_line_polyh.mlt
        prop_bool_false.mlt
        prop_str_val_empty.mlt
        prop_str_unicode.mlt
        point.mlt
        mix_3_mpoly_poly_mpoly.mlt
        mix_4_pt_line_mline_mpoly.mlt
        mix_3_line_polyh_line.mlt
        mix_4_pt_line_poly_polyh.mlt
        mix_2_poly_polyh.mlt
        mix_3_mline_pt_mline.mlt
        mix_3_polyh_line_polyh.mlt
        prop_f32_nan.mlt
        mix_2_pt_mline.mlt
        line_morton.mlt
        polygon.mlt
        mix_3_line_pt_line.mlt
        mix_4_pt_poly_polyh_mpoly.mlt
        mix_3_mline_mpoly_mline.mlt
        mix_3_poly_polyh_mline.mlt
        mix_3_mpoly_pt_mpoly.mlt
        prop_f64_max.mlt
        prop_f32_pos_inf.mlt
        prop_f64_neg_zero.mlt
        mix_3_mpoly_polyh_mpoly.mlt
        mix_4_pt_polyh_mline_mpoly.mlt
        prop_str_val_null.mlt
        id64.mlt
        props_i32_delta_rle.mlt
        ids_delta.mlt
        prop_u64.mlt
        mix_3_polyh_pt_polyh.mlt
        mix_5_pt_line_poly_mline_mpoly.mlt
        prop_f32_neg_inf.mlt
        mix_2_mline_mpoly.mlt
        extent_4096.mlt
        prop_f64_zero.mlt
        prop_i64_min.mlt
        prop_f64_min_val.mlt
        mix_4_poly_polyh_mline_mpoly.mlt
        ids64_opt_delta.mlt
        prop_bool_null_false.mlt
        mix_3_poly_line_poly.mlt
        props_i32_rle.mlt
        props_shared_dict_fsst.mlt
        prop_f32_min_norm.mlt
        props_u64.mlt
        ids_opt.mlt
        prop_i64.mlt
        prop_i32_min.mlt
        prop_f32_null_val.mlt
        polygon_hole.mlt
        mix_3_pt_line_mpt.mlt
        prop_u64_val_null.mlt
        multiline.mlt
        mix_4_pt_poly_polyh_mline.mlt
        mix_2_pt_mpoly.mlt
        prop_str_ascii.mlt
        mix_3_pt_mline_mpoly.mlt
        mix_2_mpoly_mpoly.mlt
        props_str.mlt
        mix_4_line_poly_polyh_mline.mlt
        polygon_multi.mlt
        prop_i32_null_val.mlt
        prop_f32_max.mlt
        mix_3_line_mline_line.mlt
        props_u32_delta_rle.mlt
        mix_2_mline_mline.mlt
        mix_2_polyh_polyh.mlt
        prop_f64_nan.mlt
        mix_3_pt_poly_polyh.mlt
        ids64_opt.mlt
        ids_opt_delta.mlt
        prop_f32_val_null.mlt
        extent_buf_131072.mlt
        mix_2_pt_pt.mlt
        prop_u64_null_val.mlt
        mix_3_mpoly_line_mpoly.mlt
        mix_4_line_polyh_mline_mpoly.mlt
        mix_3_pt_poly_pt.mlt
        prop_f32.mlt
        prop_i32_val_null.mlt
        prop_str_empty_val.mlt
        mix_3_poly_polyh_mpoly.mlt
        mix_2_line_polyh.mlt
        mix_3_polyh_poly_polyh.mlt
        mix_3_line_mline_mpoly.mlt
        mix_3_line_poly_line.mlt
        mix_3_poly_mpoly_poly.mlt
        mix_2_poly_poly.mlt
        mix_3_line_poly_polyh.mlt
        prop_u32_max.mlt
        mix_6_pt_line_poly_polyh_mline_mpoly.mlt
        mix_3_polyh_mline_mpoly.mlt
    )

    for f in ${MLTS[*]}; do
       "$JSON_TOOL" "$TEST_BASE/$f" >/dev/null || echo "FAILED: $f"
    done


_check-tool command:
    @which {{command}} > /dev/null || (echo "Error: {{command}} is not installed. Please install it using: brew install {{command}} (macOS) or pip3 install {{command}} (Linux)" && exit 1)
