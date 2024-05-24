#!/bin/bash
set -euo pipefail

CURRENT_DIR=$(pwd)

export CXXFLAGS="-g -std=c++17 -fPIC"

if [[ $(uname -s) = "Darwin" ]]; then
    export CXXFLAGS="${CXXFLAGS} -arch arm64 -arch x86_64"
    export JNI_INCLUDE="-I/Library/Java/JavaVirtualMachines/microsoft-17.jdk/Contents/Home/include -I/Library/Java/JavaVirtualMachines/microsoft-17.jdk/Contents/Home/include/darwin"
    export LINKFLAGS="-bundle"
else
    ls /usr/lib/jvm/
    export JNI_INCLUDE="-I/usr/lib/jvm/java-8-oracle/include"
    export LINKFLAGS="-shared"
fi

if [[ ! -f build/libfsst.a ]]; then
   mkdir -p ${CURRENT_DIR}/build
   cd ${CURRENT_DIR}/build
   if [[ ! -d cwida-fsst-ef52cb3 ]]; then
        curl -L https://github.com/cwida/fsst/tarball/ef52cb3 -o fsst.tar.gz
        tar -xvf fsst.tar.gz
        cd cwida-fsst-ef52cb3/
   fi
   cp ${CURRENT_DIR}/../../cpp/Makefile .
   make
   cp libfsst.a ${CURRENT_DIR}/build/
   cp fsst.h ${CURRENT_DIR}/build/
fi

cd ${CURRENT_DIR}

# -Wl,-rpath,'$ORIGIN' is used to set the rpath to the current directory
MODULE_BUILD="c++ ${CXXFLAGS} ${JNI_INCLUDE} -I./build -I./resources ./resources/FsstWrapper.cpp ./build/libfsst.a ${LINKFLAGS} -o ./build/FsstWrapper.so"
echo "Building FsstWrapper.so"
echo ${MODULE_BUILD}
${MODULE_BUILD}
