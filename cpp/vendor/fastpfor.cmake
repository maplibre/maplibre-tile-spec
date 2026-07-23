if(MLT_AS_OBJECT_LIBRARY)
    add_library(fastpfor-lib OBJECT)
else()
    add_library(fastpfor-lib STATIC)
endif()

target_sources(fastpfor-lib PRIVATE
    "${PROJECT_SOURCE_DIR}/vendor/fastpfor/fastpfor/bitpacking.cpp"
)

target_include_directories(fastpfor-lib SYSTEM PUBLIC
    "${PROJECT_SOURCE_DIR}/vendor/fastpfor"
)
