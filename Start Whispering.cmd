@echo off
cd /d "%~dp0dist\Whispering"
if not exist "Whispering.exe" (
    echo Whispering.exe was not found in dist\Whispering.
    echo See README.md for build instructions.
    pause
    exit /b 1
)
start "" "Whispering.exe"
