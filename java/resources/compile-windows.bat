@echo off

IF EXIST build\Release\FsstWrapper.so (
    echo "FsstWrapper.so exists, skipping compilation"
    echo "  Remove ./build/Release/FsstWrapper.so to recompile"
    echo "  Remove ./build to reconfigure & compiled"
) ELSE (
    echo "FsstWrapper.so does not exist, building now"
    mkdir resources\build
    cd resources\build
    cmake .. -DCMAKE_POLICY_VERSION_MINIMUM=3.5 -DCMAKE_BUILD_TYPE=Release
    cmake --build .
    copy FsstWrapper.so ..\..\build\FsstWrapper.so
)
IF %ERRORLEVEL% EQU 0 (
    echo "Compilation successful"
    exit /b 0
) ELSE (
    echo "Compilation failed"
    exit /b 1
)
