# Information
This wrapper is based on the work of the [FSST](https://github.com/cwida/fsst) project from cwida.

# Prerequisites
## Windows
### g++ compiler
To build the wrapper dll with the build script, a g++ compiler must be installed on your system.
Download it from [here](https://www.mingw-w64.org/downloads/), i recommend the option to download from `w64devkit`.

After the download, unzip it and add the `w64devkit/bin` directory (including the `g++.exe` file) to your path environment variable.

### Java
A Java Runtime Environment with version >= 17 has to be installed on the system.

## Building the wrapper
### Steps to build for windows
1. Check out the FSST project and build the dll for windows
2. Copy the "fsst.dll" into fsst-wrapper/resources
3. Run a gradle build to build the "FsstWrapper.dll" using the "compileWrapper" gradle task
