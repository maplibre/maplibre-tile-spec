cmake_minimum_required (VERSION 3.12)

project (mlt-cpp-test)

add_executable(mlt-cpp-test
    decode.cpp
    fastpfor.cpp
)
set_property(TARGET mlt-cpp-test PROPERTY CXX_STANDARD 20)
target_compile_definitions(mlt-cpp-test PRIVATE PROTOZERO_USE_VIEW=std::string_view)
target_include_directories(mlt-cpp-test PRIVATE "${protozero_SOURCE_DIR}/include")

# Why doesn't protozero set up its own include directory?
target_include_directories(pbf-decoder PRIVATE "${protozero_SOURCE_DIR}/include")
target_include_directories(reader_tests PRIVATE "${protozero_SOURCE_DIR}/include")
target_include_directories(writer_tests PRIVATE "${protozero_SOURCE_DIR}/include")
target_include_directories(unit_tests PRIVATE "${protozero_SOURCE_DIR}/include")

include(FetchContent)
set(BUILD_GMOCK OFF)
set(INSTALL_GTEST OFF)
set(gtest_force_shared_crt ON)
FetchContent_Declare(
    gtest
    GIT_REPOSITORY https://github.com/google/googletest.git
    GIT_TAG v1.15.2
)
# For Windows: Prevent overriding the parent project's compiler/linker settings
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
FetchContent_MakeAvailable(gtest)
target_include_directories(mlt-cpp-test PRIVATE "${gtest_SOURCE_DIR}/googletest/include")

target_include_directories(mlt-cpp-test PRIVATE "${fastpfor_SOURCE_DIR}/headers")

target_link_libraries(mlt-cpp-test mlt-cpp gtest_main FastPFOR)
#gtest_discover_tests(mlt-cpp-test)

