set(TEST_SOURCES
    test_decode.cpp
    test_fastpfor.cpp
    test_fsst.cpp
    test_hilbert_curve.cpp
    test_morton_curve.cpp
    test_packed_bitset.cpp
    test_type_map.cpp
    test_util.cpp
    test_varint.cpp
)

# mlt-cpp and mlt-cpp-encoder both contain the metadata sources. Object libraries put every one of their objects in a
# link, so a binary that uses both of them has those twice. Leave the encoder tests out then.
set(ENCODER_TEST_SOURCES
    test_encode.cpp
    test_synthetic.cpp
    test_winding.cpp
)
if(MLT_AS_OBJECT_LIBRARY)
    message(STATUS "[MLT] Not building the encoder tests with MLT_AS_OBJECT_LIBRARY")
else()
    list(APPEND TEST_SOURCES ${ENCODER_TEST_SOURCES})
endif()

add_executable(mlt-cpp-test ${TEST_SOURCES})
set_property(TARGET mlt-cpp-test PROPERTY CXX_STANDARD 20)

# Tests have access to private headers
target_include_directories(mlt-cpp-test
    PUBLIC "${PROJECT_SOURCE_DIR}/include"
    PRIVATE "${PROJECT_SOURCE_DIR}/src"
)

option(MLT_WITH_JSON "Include JSON support" ON)

if(MLT_WITH_JSON)
    target_include_directories(mlt-cpp-test PUBLIC "${json_SOURCE_DIR}/include")
    target_compile_definitions(mlt-cpp-test PUBLIC MLT_WITH_JSON=1)
endif(MLT_WITH_JSON)

if (MLT_WITH_CLANG_TIDY)
    set_target_properties(mlt-cpp-test PROPERTIES CXX_CLANG_TIDY "${CLANG_TIDY_COMMAND}")
endif()

set(BUILD_GMOCK OFF CACHE BOOL "" FORCE)
set(INSTALL_GTEST OFF CACHE BOOL "" FORCE)

# For Windows: Prevent overriding the parent project's compiler/linker settings
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)

add_subdirectory("${PROJECT_SOURCE_DIR}/vendor/googletest" "${CMAKE_CURRENT_BINARY_DIR}/googletest" EXCLUDE_FROM_ALL SYSTEM)

if(MLT_AS_OBJECT_LIBRARY)
    target_link_libraries(mlt-cpp-test mlt-cpp fastpfor-lib gtest_main)
else()
    target_link_libraries(mlt-cpp-test mlt-cpp mlt-cpp-encoder fastpfor-lib gtest_main)
endif()

include(GoogleTest)

gtest_discover_tests(
    mlt-cpp-test
    WORKING_DIRECTORY ${PROJECT_SOURCE_DIR}
)
