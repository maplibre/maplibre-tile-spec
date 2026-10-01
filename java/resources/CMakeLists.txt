cmake_minimum_required(VERSION 3.5)

project(fsst)

find_package(Threads REQUIRED)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(CMAKE_VERBOSE_MAKEFILE ON)

if(APPLE)
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -arch x86_64 -arch arm64")
endif(APPLE)

if (LINUX)
  include(CheckCXXCompilerFlag)
  check_cxx_compiler_flag("-march=native" COMPILER_SUPPORTS_MARCH_NATIVE)
  if(COMPILER_SUPPORTS_MARCH_NATIVE)
      set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -march=native")
  endif()
endif(LINUX)

if(NOT CMAKE_BUILD_TYPE)
 set(CMAKE_BUILD_TYPE Release)
endif()

if(MSVC)
    set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} /O3 /DNDEBUG")
else()
    set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} -O3 -DNDEBUG")
endif()

find_package(JNI)
if (JNI_FOUND)
    message (STATUS "JNI_INCLUDE_DIRS=${JNI_INCLUDE_DIRS}")
    message (STATUS "JNI_LIBRARIES=${JNI_LIBRARIES}")
endif()

include(ExternalProject)
ExternalProject_Add(FsstDownload
    GIT_REPOSITORY https://github.com/springmeyer/fsst
    GIT_TAG bd766bc # https://github.com/springmeyer/fsst/tree/dane/cross-platform-build-fixes
    INSTALL_COMMAND ""
    PREFIX ${CMAKE_CURRENT_BINARY_DIR}/libfsst-prefix
    CMAKE_ARGS "-DCMAKE_CXX_FLAGS=${CMAKE_CXX_FLAGS}"
)
SET(LIBFSST_LIB_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/libfsst-prefix/src/FsstDownload-build)
SET(LIBFSST_INC_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/libfsst-prefix/src/FsstDownload)

link_directories(${LIBFSST_LIB_DIRECTORY})

add_library(libfsst STATIC IMPORTED)

# TODO: find a way to avoid having to be explicit about library name differences between platforms
if(MSVC)
    set_target_properties(libfsst PROPERTIES IMPORTED_LOCATION ${LIBFSST_LIB_DIRECTORY}/Release/fsst.lib)
else()
    set_target_properties(libfsst PROPERTIES IMPORTED_LOCATION ${LIBFSST_LIB_DIRECTORY}/libfsst.a)
endif()


add_library(FsstWrapper MODULE ${CMAKE_CURRENT_SOURCE_DIR}/FsstWrapper.cpp)
add_dependencies(FsstWrapper FsstDownload)
set_target_properties(FsstWrapper PROPERTIES
    OUTPUT_NAME "${output_name}"
    POSITION_INDEPENDENT_CODE ON
    SUFFIX ".so"
    PREFIX ""
)
target_include_directories(FsstWrapper PUBLIC ${CMAKE_CURRENT_SOURCE_DIR} ${JNI_INCLUDE_DIRS} ${LIBFSST_INC_DIRECTORY})
target_link_libraries(FsstWrapper libfsst)
