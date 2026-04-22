@echo off
REM Cortex dev launcher — sets up MSVC environment then runs Tauri dev
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
set PATH=C:\Users\PC\.cargo\bin;%PATH%
npm run tauri dev
