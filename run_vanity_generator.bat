@echo off
powershell -Command "Start-Process powershell -Verb RunAs -ArgumentList '-NoExit', '-Command', 'cd \"%~dp0\"; .\target\release\rust_vanity_generator.exe'" 