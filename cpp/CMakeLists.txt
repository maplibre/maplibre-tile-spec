cmake_minimum_required (VERSION 3.12)

project (mlt-cpp)

add_library(mlt-cpp STATIC)
set_property(TARGET mlt-cpp PROPERTY CXX_STANDARD 20)
target_compile_definitions(mlt-cpp PUBLIC PROTOZERO_USE_VIEW=std::string_view)

target_include_directories(mlt-cpp
    PUBLIC ${PROJECT_SOURCE_DIR}/include
)

list(APPEND INCLUDE_FILES
    ${PROJECT_SOURCE_DIR}/include/tileset_metadata.hpp
)

list(APPEND SRC_FILES
    ${PROJECT_SOURCE_DIR}/src/tileset_metadata.cpp
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

