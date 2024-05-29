@echo off
echo "Compiling on windows"
echo %cd%
bash resources/compile
IF %ERRORLEVEL% EQU 0 (
    echo "Compilation successful"
    exit /b 0
) ELSE (
    echo "Compilation failed"
    exit /b 1
)
