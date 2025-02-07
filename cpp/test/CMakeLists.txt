cmake_minimum_required (VERSION 3.12)

project (mlt-cpp-test)

add_executable(mlt-cpp-test
    test_decode.cpp
    test_fastpfor.cpp
    test_fsst.cpp
    test_packed_bitset.cpp
    test_util.cpp
)
set_property(TARGET mlt-cpp-test PROPERTY CXX_STANDARD 20)

# Tests have access to private headers
target_include_directories(mlt-cpp-test PUBLIC "${PROJECT_SOURCE_DIR}/../src")

option(MLT_WITH_JSON "Include JSON support" ON)
if(MLT_WITH_JSON)
    target_include_directories(mlt-cpp-test PUBLIC "${json_SOURCE_DIR}/include")
    target_compile_definitions(mlt-cpp-test PUBLIC MLT_WITH_JSON=1)
endif(MLT_WITH_JSON)

target_compile_definitions(mlt-cpp-test PRIVATE PROTOZERO_USE_VIEW=std::string_view)

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

target_include_directories(mlt-cpp-test PRIVATE "${fastpfor_SOURCE_DIR}/headers" "${fsst_SOURCE_DIR}")

target_link_libraries(mlt-cpp-test PRIVATE mlt-cpp gtest_main FastPFOR fsst protozero)
