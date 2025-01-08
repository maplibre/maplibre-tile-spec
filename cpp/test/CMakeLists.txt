cmake_minimum_required (VERSION 3.12)

project (mlt_cpp_test)

include(FetchContent)

set(BUILD_GMOCK OFF)
set(INSTALL_GTEST OFF)
set(gtest_force_shared_crt ON)
FetchContent_Declare(
    GTest
    GIT_REPOSITORY https://github.com/google/googletest.git
    GIT_TAG v1.15.2
)
# For Windows: Prevent overriding the parent project's compiler/linker settings
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(GTest)

#option(BUILD_TESTING "" OFF)
FetchContent_Declare(
    protozero
    GIT_REPOSITORY https://github.com/mapbox/protozero.git
    GIT_TAG v1.7.1
)
FetchContent_MakeAvailable(protozero)

# Why doesn't protozero set up its own include directory?
target_include_directories(pbf-decoder PRIVATE "${protozero_SOURCE_DIR}/include")
target_include_directories(reader_tests PRIVATE "${protozero_SOURCE_DIR}/include")
target_include_directories(writer_tests PRIVATE "${protozero_SOURCE_DIR}/include")
target_include_directories(unit_tests PRIVATE "${protozero_SOURCE_DIR}/include")

add_executable(mlt_test test_decode.cpp)
set_property(TARGET mlt_test PROPERTY CXX_STANDARD 20)
target_include_directories(mlt_test PRIVATE "${protozero_SOURCE_DIR}/include")
target_link_libraries(mlt_test gtest_main)
#gtest_discover_tests(mlt_test)

