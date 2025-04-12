@echo off
title Bitcoin Vanity Address Generator

:: Get the directory where this batch file is located
set "SCRIPT_DIR=%~dp0"

:: Check if the executable exists
if not exist "%SCRIPT_DIR%target\release\rust_vanity_generator.exe" (
    echo Error: rust_vanity_generator.exe not found!
    echo Please make sure you have built the program with: cargo build --release
    pause
    exit /b 1
)

:: Run the program in an elevated PowerShell
powershell -Command "Start-Process powershell -Verb RunAs -ArgumentList '-NoExit', '-Command', 'cd \"%SCRIPT_DIR%\"; .\target\release\rust_vanity_generator.exe'" 