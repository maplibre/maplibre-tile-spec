add_executable(mlt-cpp-json mlt-json.cpp)
set_property(TARGET mlt-cpp-json PROPERTY CXX_STANDARD 20)
set(MLT_WITH_JSON ON CACHE BOOL "Include JSON Support" FORCE)
target_link_libraries(mlt-cpp-json PRIVATE mlt-cpp)

# FastPFOR vector generator for cross-language testing
if(MLT_WITH_FASTPFOR)
    add_executable(generate-fastpfor-vectors generate-fastpfor-vectors.cpp)
    set_property(TARGET generate-fastpfor-vectors PROPERTY CXX_STANDARD 20)
    target_link_libraries(generate-fastpfor-vectors PRIVATE FastPFOR)
    target_include_directories(generate-fastpfor-vectors PRIVATE SYSTEM "${PROJECT_SOURCE_DIR}/vendor/fastpfor/headers")
endif()
