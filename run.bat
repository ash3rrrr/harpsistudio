@echo off
setlocal
cd /d "%~dp0"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

where cargo >nul 2>nul
if errorlevel 1 (
    echo Rust was not found. Install it from https://rustup.rs and try again.
    pause
    exit /b 1
)

cargo run --release
if errorlevel 1 pause
