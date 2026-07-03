@echo off
setlocal
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat" > nul
cl /nologo /utf-8 /O2 /EHsc /LD /Fe:"..\target\debug\jc9_taskbar.dll" taskbar_dll.cpp ole32.lib shell32.lib user32.lib gdi32.lib dwmapi.lib /link /DLL /MACHINE:X64
endlocal
