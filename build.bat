@echo off
setlocal enabledelayedexpansion

rem Builds every Rust crate in the project, stopping at the first failure.

set "ROOT=%~dp0"
set "FAILED="

call :build "%ROOT%rust" root
call :build "%ROOT%addons\growth\rust" addons\growth
call :build "%ROOT%addons\beatsmr\rust" addons\beatsmr

if defined FAILED (
    echo.
    echo BUILD FAILED: !FAILED!
    exit /b 1
)

echo.
echo All crates built successfully.
exit /b 0

:build
echo.
echo === cargo build %~2 ===
pushd "%~1"
if errorlevel 1 goto :build_enter_failed
cargo build
set "RESULT=%ERRORLEVEL%"
popd
if not "%RESULT%"=="0" (
    set "FAILED=%~2"
    exit /b 1
)
exit /b 0

:build_enter_failed
echo Cannot enter %~1
set "FAILED=%~2"
exit /b 1
