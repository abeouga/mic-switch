@echo off
rem Double-click to build and install Mic Switch. Arguments are passed to setup.ps1.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0setup.ps1" %*
pause
