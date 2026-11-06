# libFuzzer targets, which need Clang. For example:
#
#   CXX=clang++ cmake -B build-fuzz -S . -DCMAKE_BUILD_TYPE=RelWithDebInfo \
#       -DMLT_WITH_FUZZERS=ON -DMLT_WITH_TESTS=OFF -DMLT_WITH_TOOLS=OFF \
#       -DCMAKE_CXX_FLAGS="-fsanitize=fuzzer-no-link,address,undefined -fno-sanitize-recover=undefined"
#   cmake --build build-fuzz
#   mkdir corpus && build-fuzz/fuzz/mlt-cpp-fuzz-tile corpus ../test/expected/0x01/simple -max_len=65536
#
# Each target takes a corpus directory (see `--help` of libFuzzer for the rest of the options).
if(NOT CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    message(FATAL_ERROR "MLT_WITH_FUZZERS requires Clang")
endif()

foreach(_name fastpfor tile)
    set(_target mlt-cpp-fuzz-${_name})
    add_executable(${_target} fuzz_${_name}.cpp)
    set_property(TARGET ${_target} PROPERTY CXX_STANDARD 20)
    target_include_directories(${_target} PRIVATE "${PROJECT_SOURCE_DIR}/src")
    target_link_libraries(${_target} PRIVATE mlt-cpp)
    target_compile_options(${_target} PRIVATE -fsanitize=fuzzer)
    target_link_options(${_target} PRIVATE -fsanitize=fuzzer)
endforeach()
