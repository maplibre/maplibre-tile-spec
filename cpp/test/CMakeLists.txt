set(TEST_SOURCES
    test_decode.cpp
    test_fsst.cpp
    test_packed_bitset.cpp
    test_util.cpp
    test_varint.cpp
)

if (MLT_WITH_FASTPFOR)
    list(APPEND TEST_SOURCES test_fastpfor.cpp)
endif()

add_executable(mlt-cpp-test ${TEST_SOURCES})
set_property(TARGET mlt-cpp-test PROPERTY CXX_STANDARD 20)

# Tests have access to private headers
target_include_directories(mlt-cpp-test PUBLIC "${PROJECT_SOURCE_DIR}/src")

option(MLT_WITH_JSON "Include JSON support" ON)

if(MLT_WITH_JSON)
    target_include_directories(mlt-cpp-test PUBLIC "${json_SOURCE_DIR}/include")
    target_compile_definitions(mlt-cpp-test PUBLIC MLT_WITH_JSON=1)
endif(MLT_WITH_JSON)
if(MLT_WITH_FASTPFOR)
    target_include_directories(mlt-cpp-test PRIVATE SYSTEM "${PROJECT_SOURCE_DIR}/vendor/fastpfor/headers")
endif(MLT_WITH_FASTPFOR)
target_include_directories(mlt-cpp-test PRIVATE SYSTEM "${PROJECT_SOURCE_DIR}/vendor/fsst")

set(BUILD_GMOCK OFF CACHE BOOL "" FORCE)
set(INSTALL_GTEST OFF CACHE BOOL "" FORCE)

# For Windows: Prevent overriding the parent project's compiler/linker settings
set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)

add_subdirectory("${PROJECT_SOURCE_DIR}/vendor/googletest" "${CMAKE_CURRENT_BINARY_DIR}/googletest" EXCLUDE_FROM_ALL SYSTEM)

target_link_libraries(mlt-cpp-test mlt-cpp gtest_main)
