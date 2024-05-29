@echo off
echo "Compiling on windows"
echo %cd%
cd /d "%~dp0"
echo %cd%
bash compile
IF %ERRORLEVEL% EQU 0 (
    echo "Compilation successful"
    exit /b 0
) ELSE (
    echo "Compilation failed"
    exit /b 1
)
