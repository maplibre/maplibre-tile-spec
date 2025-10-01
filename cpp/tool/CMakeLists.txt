add_executable(mlt-cpp-json mlt-geojson.cpp)
set_property(TARGET mlt-cpp-json PROPERTY CXX_STANDARD 20)
set(MLT_WITH_JSON ON CACHE BOOL "Include JSON Support" FORCE)
target_link_libraries(mlt-cpp-json PRIVATE mlt-cpp)
