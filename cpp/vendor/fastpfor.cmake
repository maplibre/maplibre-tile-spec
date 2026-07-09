add_library(fastpfor-lib STATIC
    "${PROJECT_SOURCE_DIR}/vendor/fastpfor/fastpfor/bitpacking.cpp"
)

target_include_directories(fastpfor-lib SYSTEM PUBLIC
    "${PROJECT_SOURCE_DIR}/vendor/fastpfor"
)
