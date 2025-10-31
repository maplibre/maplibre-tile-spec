@echo off

IF EXIST resources\build\Release\FsstWrapper.so (
    echo "FsstWrapper.so exists, skipping compilation"
    echo "  Remove ./resources/build/Release/FsstWrapper.so to recompile"
) ELSE (
    echo "FsstWrapper.so does not exist, building now"
    mkdir resources\build
    cd resources\build
    cmake .. -DCMAKE_POLICY_VERSION_MINIMUM=3.5 -DCMAKE_BUILD_TYPE=Release
    cmake --build .
    copy /y Release\FsstWrapper.so
)
IF %ERRORLEVEL% EQU 0 (
    echo "Compilation successful"
    exit /b 0
) ELSE (
    echo "Compilation failed"
    exit /b 1
)
