@echo off
cd /d "%~dp0"
g++ -std=c++11 -shared -I. -I..\resources -L. -lfsst FsstWrapper.cpp -o FsstWrapper.dll -lstdc++ -Wl,-rpath
IF %ERRORLEVEL% EQU 0 (
    copy fsst.dll ..\.
    copy FsstWrapper.dll ..\.
)