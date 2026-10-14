@echo off
setlocal
for /f "delims=" %%D in ('dir /b /ad /o-d "%~dp0dist\staging-*" 2^>nul') do (
    if exist "%~dp0dist\%%D\Whispering\SHA256SUMS.txt" (
        if exist "%~dp0dist\%%D\Whispering\Whispering.exe" (
            start "" /d "%~dp0dist\%%D\Whispering" "%~dp0dist\%%D\Whispering\Whispering.exe"
            exit /b 0
        )
    )
)
echo No staged Whispering bundle found. See README.md for build instructions.
pause
exit /b 1
