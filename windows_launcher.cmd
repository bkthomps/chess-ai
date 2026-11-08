@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\launch.ps1" %*
if errorlevel 1 (
    echo.
    echo Launch failed. Review the message above.
    pause
    exit /b 1
)
