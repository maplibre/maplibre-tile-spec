cmake_minimum_required (VERSION 3.12)

project (mlt-cpp)

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
)

list(APPEND SRC_FILES
    ${PROJECT_SOURCE_DIR}/src/decoder.cpp
    ${PROJECT_SOURCE_DIR}/src/metadata/stream.cpp
    ${PROJECT_SOURCE_DIR}/src/metadata/tileset.cpp
    ${PROJECT_SOURCE_DIR}/src/util/float.hpp
    ${PROJECT_SOURCE_DIR}/src/util/morton.hpp
    ${PROJECT_SOURCE_DIR}/src/util/pfor.hpp
    ${PROJECT_SOURCE_DIR}/src/util/rle.hpp
    ${PROJECT_SOURCE_DIR}/src/util/varint.hpp
    ${PROJECT_SOURCE_DIR}/src/util/zigzag.hpp
)

target_sources(mlt-cpp PRIVATE
    ${INCLUDE_FILES}
    ${SRC_FILES}
)

include(FetchContent)
fetchcontent_declare(protozero
    git_repository https://github.com/mapbox/protozero.git
    git_tag v1.7.1
)
FetchContent_MakeAvailable(protozero)
target_include_directories(mlt-cpp PRIVATE "${protozero_SOURCE_DIR}/include")

add_subdirectory(${PROJECT_SOURCE_DIR}/test)

