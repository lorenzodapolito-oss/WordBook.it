@echo off
rem Builds WordBook.it and copies it as dist\WordBook.it\WordBook.it.exe.
rem Put the MSYS2/WinLibs compiler folder on PATH first, e.g.
rem   set PATH=C:\msys64\ucrt64\bin;%PATH%
cd /d "%~dp0\..\.."
cargo build --release -p wordcraft
if errorlevel 1 exit /b 1
if not exist dist\WordBook.it mkdir dist\WordBook.it
copy /Y target\release\wordbook.exe dist\WordBook.it\WordBook.it.exe
echo.
echo Done: dist\WordBook.it\WordBook.it.exe
echo For portable mode, create a folder named wordbook-data next to it.
