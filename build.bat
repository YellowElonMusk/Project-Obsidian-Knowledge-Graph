@echo off
REM Cortex production build
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
set PATH=C:\Users\PC\.cargo\bin;%PATH%
npm run tauri build
