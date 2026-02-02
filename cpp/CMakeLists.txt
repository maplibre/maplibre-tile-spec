cmake_minimum_required(VERSION 3.25)
project(mlt-cpp LANGUAGES CXX)

list(APPEND CMAKE_MODULE_PATH ${CMAKE_CURRENT_LIST_DIR}/cmake)
include(AddCXXCompilerFlag)

# Use the same deployment target as the parent project, if set.
if (NOT MLT_OSX_DEPLOYMENT_TARGET)
    set(MLT_OSX_DEPLOYMENT_TARGET 16)
endif()
set(CMAKE_OSX_DEPLOYMENT_TARGET ${MLT_OSX_DEPLOYMENT_TARGET})

set(CMAKE_CXX_STANDARD 20)
add_library(mlt-cpp STATIC)

option(MLT_WITH_JSON "Include JSON support" ON)
option(MLT_WITH_FASTPFOR "Include FastPFor support" ON)
option(MLT_WITH_TESTS "Include Tests" ON)
option(MLT_WITH_TOOLS "Include CLI tools" ON)

set_target_properties(
    mlt-cpp
    PROPERTIES
    INTERFACE_MAPLIBRE_NAME "MapLibre Tile Format"
    INTERFACE_MAPLIBRE_URL "https://github.com/maplibre/maplibre-tile-spec"
    INTERFACE_MAPLIBRE_AUTHOR "MapLibre"
    INTERFACE_MAPLIBRE_LICENSE "${PROJECT_SOURCE_DIR}/../LICENSE-APACHE"
)

add_cxx_compiler_flag(-Wall)
add_cxx_compiler_flag(-Werror)
add_cxx_compiler_flag(-Wextra)

add_cxx_compiler_flag(-Wdeprecated-declarations)
add_cxx_compiler_flag(-Winvalid-offsetof)
add_cxx_compiler_flag(-Wno-block-capture-autoreleasing)
add_cxx_compiler_flag(-Wno-bool-conversion)
add_cxx_compiler_flag(-Wno-c++11-extensions)
add_cxx_compiler_flag(-Wno-comma)
add_cxx_compiler_flag(-Wno-constant-conversion)
add_cxx_compiler_flag(-Wno-conversion)
add_cxx_compiler_flag(-Wno-empty-body)
add_cxx_compiler_flag(-Wno-enum-conversion)
add_cxx_compiler_flag(-Wno-exit-time-destructors)
add_cxx_compiler_flag(-Wno-float-conversion)
add_cxx_compiler_flag(-Wno-four-char-constants)
add_cxx_compiler_flag(-Wno-implicit-fallthrough)
add_cxx_compiler_flag(-Wno-infinite-recursion)
add_cxx_compiler_flag(-Wno-missing-braces)
add_cxx_compiler_flag(-Wno-missing-field-initializers)
add_cxx_compiler_flag(-Wno-move)
add_cxx_compiler_flag(-Wno-newline-eof)
add_cxx_compiler_flag(-Wno-non-literal-null-conversion)
add_cxx_compiler_flag(-Wno-non-virtual-dtor)
add_cxx_compiler_flag(-Wno-objc-literal-conversion)
add_cxx_compiler_flag(-Wno-overloaded-virtual)
add_cxx_compiler_flag(-Wno-range-loop-analysis)
add_cxx_compiler_flag(-Wno-return-type)
add_cxx_compiler_flag(-Wno-semicolon-before-method-body)
add_cxx_compiler_flag(-Wno-shadow)
add_cxx_compiler_flag(-Wno-sign-conversion)
add_cxx_compiler_flag(-Wno-trigraphs)
add_cxx_compiler_flag(-Wno-uninitialized)
add_cxx_compiler_flag(-Wno-unknown-pragmas)
add_cxx_compiler_flag(-Wno-unused-function)
add_cxx_compiler_flag(-Wno-unused-label)
add_cxx_compiler_flag(-Wno-unused-parameter)
add_cxx_compiler_flag(-Wno-unused-variable)
add_cxx_compiler_flag(-Wparentheses)
add_cxx_compiler_flag(-Wshorten-64-to-32)
add_cxx_compiler_flag(-Wswitch)
add_cxx_compiler_flag(-Wunused-value)
add_cxx_compiler_flag(-fstrict-aliasing)
add_cxx_compiler_flag(-wd4061) # MSVC: enum member not handled in switch
add_cxx_compiler_flag(-wd4514) # MSVC: unreferenced inline function has been removed
add_cxx_compiler_flag(-wd4710) # MSVC: function not inlined
add_cxx_compiler_flag(-wd4820) # MSVC: padding added after data member

target_include_directories(mlt-cpp
    PUBLIC ${PROJECT_SOURCE_DIR}/include
    PRIVATE ${PROJECT_SOURCE_DIR}/src
)

list(APPEND MLT_INCLUDE_FILES
    ${PROJECT_SOURCE_DIR}/include/mlt/common.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/decoder.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/feature.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/geometry.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/geometry_vector.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/layer.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/polyfill.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/projection.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/properties.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/tile.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/metadata/stream.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/metadata/tileset.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/util/buffer_stream.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/util/packed_bitset.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/util/noncopyable.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/util/stl.hpp
    ${PROJECT_SOURCE_DIR}/include/mlt/util/varint.hpp
)

list(APPEND MLT_SRC_FILES
    ${PROJECT_SOURCE_DIR}/src/mlt/decoder.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/geometry.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/int.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/int.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/property.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/decode/string.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/feature.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/geometry_vector.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/layer.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/metadata/stream.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/metadata/tileset.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/properties.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/morton_curve.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/raw.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/rle.cpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/rle.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/space_filling_curve.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/vectorized.hpp
    ${PROJECT_SOURCE_DIR}/src/mlt/util/zigzag.hpp
)

if(MLT_WITH_JSON)
    list(APPEND MLT_INCLUDE_FILES ${PROJECT_SOURCE_DIR}/include/mlt/json.hpp)
endif(MLT_WITH_JSON)

target_sources(mlt-cpp PRIVATE
    ${MLT_INCLUDE_FILES}
    ${MLT_SRC_FILES}
)

# SIMD Everywhere (needed on ARM)
target_include_directories(mlt-cpp PRIVATE "${PROJECT_SOURCE_DIR}/vendor/simde")

if(MLT_WITH_FASTPFOR)
    message(STATUS "[MLT] Including FastPFOR support")

    # FastPFor
    set(FASTPFOR_WITH_TEST OFF CACHE BOOL "Disable tests in FastPFor" FORCE) # The fastpfor gtest targets conflict with ours

    # SUPPORT_NEON results in trying to link simde (`-lsimde`) but it's a header-only library
    set(SUPPORT_NEON OFF CACHE BOOL "" FORCE)

    add_subdirectory("${PROJECT_SOURCE_DIR}/vendor/fastpfor" "${CMAKE_CURRENT_BINARY_DIR}/fastpfor" EXCLUDE_FROM_ALL SYSTEM)

    # Disable all warnings for FastPFOR
    if(MSVC)
        target_compile_options(FastPFOR PRIVATE /w)
    else()
        target_compile_options(FastPFOR PRIVATE -w)
    endif()

    target_link_libraries(mlt-cpp FastPFOR)
    target_include_directories(mlt-cpp PRIVATE SYSTEM "${PROJECT_SOURCE_DIR}/vendor/fastpfor/headers")
    target_compile_definitions(mlt-cpp PUBLIC MLT_ENABLE_FASTPFOR=1)
    list(APPEND MLT_EXPORT_TARGETS FastPFOR)
else()
    message(STATUS "[MLT] No FastPFOR support")
endif(MLT_WITH_FASTPFOR)

# json
if(MLT_WITH_JSON)
    message(STATUS "[MLT] Including JSON support")

    add_subdirectory("${PROJECT_SOURCE_DIR}/vendor/json" "${CMAKE_CURRENT_BINARY_DIR}/json" EXCLUDE_FROM_ALL SYSTEM)

    set(json_SOURCE_DIR "${PROJECT_SOURCE_DIR}/vendor/json")
    target_include_directories(mlt-cpp PUBLIC "${json_SOURCE_DIR}/include")
    target_compile_definitions(mlt-cpp PUBLIC MLT_WITH_JSON=1)
else()
    message(STATUS "[MLT] No JSON support")
endif(MLT_WITH_JSON)

if(MLT_WITH_TESTS)
    add_subdirectory(${PROJECT_SOURCE_DIR}/test EXCLUDE_FROM_ALL)
endif(MLT_WITH_TESTS)

if(MLT_WITH_TOOLS)
    add_subdirectory(${PROJECT_SOURCE_DIR}/tool EXCLUDE_FROM_ALL)
endif(MLT_WITH_TOOLS)

export(TARGETS ${MLT_EXPORT_TARGETS} FILE MLTTargets.cmake)
