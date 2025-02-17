cmake_minimum_required(VERSION 3.12)
project(mlt-cpp-test)

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

set(BUILD_GMOCK OFF CACHE BOOL "" FORCE)
set(INSTALL_GTEST OFF CACHE BOOL "" FORCE)

# For Windows: Prevent overriding the parent project's compiler/linker settings
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)

if(CMAKE_VERSION VERSION_GREATER_EQUAL "3.25")
    add_subdirectory("${PROJECT_SOURCE_DIR}/../../vendor/googletest" "${CMAKE_CURRENT_BINARY_DIR}/../googletest" SYSTEM)
else()
    add_subdirectory("${PROJECT_SOURCE_DIR}/../../vendor/googletest" "${CMAKE_CURRENT_BINARY_DIR}/../googletest")
endif()

target_link_libraries(mlt-cpp-test mlt-cpp gtest_main)
