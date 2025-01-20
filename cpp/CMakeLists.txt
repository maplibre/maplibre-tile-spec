include(FetchContent)

cmake_minimum_required (VERSION 3.12)
set(CMAKE_OSX_DEPLOYMENT_TARGET 15)
project (mlt-cpp LANGUAGES CXX)

add_library(mlt-cpp STATIC)
set_property(TARGET mlt-cpp PROPERTY CXX_STANDARD 20)
target_compile_definitions(mlt-cpp PUBLIC PROTOZERO_USE_VIEW=std::string_view)

target_include_directories(mlt-cpp
    PUBLIC ${PROJECT_SOURCE_DIR}/include
    PRIVATE ${PROJECT_SOURCE_DIR}/src
)

list(APPEND INCLUDE_FILES
    ${PROJECT_SOURCE_DIR}/include/common.hpp
    ${PROJECT_SOURCE_DIR}/include/decoder.hpp
    ${PROJECT_SOURCE_DIR}/include/feature.hpp
    ${PROJECT_SOURCE_DIR}/include/layer.hpp
    ${PROJECT_SOURCE_DIR}/include/tile.hpp
    ${PROJECT_SOURCE_DIR}/include/metadata/stream.hpp
    ${PROJECT_SOURCE_DIR}/include/metadata/tileset.hpp
    ${PROJECT_SOURCE_DIR}/include/util/buffer_stream.hpp
)

list(APPEND SRC_FILES
    ${PROJECT_SOURCE_DIR}/src/decoder.cpp
    ${PROJECT_SOURCE_DIR}/src/decode/ints.hpp
    ${PROJECT_SOURCE_DIR}/src/decode/ints.cpp
    ${PROJECT_SOURCE_DIR}/src/metadata/stream.cpp
    ${PROJECT_SOURCE_DIR}/src/metadata/tileset.cpp
    ${PROJECT_SOURCE_DIR}/src/util/float.hpp
    ${PROJECT_SOURCE_DIR}/src/util/morton.hpp
    ${PROJECT_SOURCE_DIR}/src/util/rle.hpp
    ${PROJECT_SOURCE_DIR}/src/util/varint.hpp
    ${PROJECT_SOURCE_DIR}/src/util/zigzag.hpp
)

target_sources(mlt-cpp PRIVATE
    ${INCLUDE_FILES}
    ${SRC_FILES}
)

# Protozero
fetchcontent_declare(protozero
    GIT_REPOSITORY https://github.com/mapbox/protozero.git
    GIT_TAG v1.7.1
)
FetchContent_MakeAvailable(protozero)
target_include_directories(mlt-cpp PRIVATE "${protozero_SOURCE_DIR}/include")

# FastPFor
set(WITH_TEST OFF)  # The fastpfor gtest targets conflict with ours
fetchcontent_declare(fastpfor
    GIT_REPOSITORY https://github.com/fast-pack/FastPFor.git
    GIT_TAG v0.2.0
)
FetchContent_MakeAvailable(fastpfor)
target_include_directories(mlt-cpp PRIVATE "${fastpfor_SOURCE_DIR}/headers")
target_compile_options(FastPFOR PRIVATE "-Wno-cast-align")

add_subdirectory(${PROJECT_SOURCE_DIR}/test)

