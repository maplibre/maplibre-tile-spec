@echo off
echo %cd%IF EXIST build\FsstWrapper.so (
    echo "FsstWrapper.so exists, skipping compilation"
    echo "  Remove ./build/FsstWrapper.so to recompile"
    echo "  Remove ./build to reconfigure & compiled"
) ELSE (
    echo "FsstWrapper.so does not exist, building now"
    mkdir -p build
    cd build
    cmake ../Resources
    cmake --build . --config Release
)
IF %ERRORLEVEL% EQU 0 (
    echo "Compilation successful"
    exit /b 0
) ELSE (
    echo "Compilation failed"
    exit /b 1
)
