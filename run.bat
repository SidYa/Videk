@echo off
rem Videk launcher (keep this file ASCII with CRLF line endings: cmd.exe misparses others).
rem   run.bat           - menu
rem   run.bat 1 / dev   - run from the sources (tauri dev), no installer
rem   run.bat 2 / nsis  - build the NSIS installer (release)
setlocal
cd /d "%~dp0"

set "MODE=%~1"
if /i "%MODE%"=="dev" set "MODE=1"
if /i "%MODE%"=="nsis" set "MODE=2"
if not "%MODE%"=="" goto :check

echo.
echo   Videk
echo   1. Run (no installer)
echo   2. Build NSIS installer
echo.
rem One key press, no Enter.
choice /c 12 /n /m "Press 1 or 2: "
set "MODE=%errorlevel%"

:check
where pnpm >nul 2>nul
if errorlevel 1 (
  echo pnpm not found. Install it: npm install -g pnpm
  goto :fail
)
where cargo >nul 2>nul
if errorlevel 1 (
  echo cargo not found. Install Rust: https://rustup.rs
  goto :fail
)

if not exist node_modules (
  echo Installing dependencies...
  call pnpm install
  if errorlevel 1 goto :fail
)

if not exist vendor\mpv\libmpv-2.dll (
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\fetch-libmpv.ps1
  if errorlevel 1 goto :fail
)

if "%MODE%"=="1" goto :dev
if "%MODE%"=="2" goto :nsis
echo Unknown choice: %MODE%
goto :fail

:dev
call pnpm tauri dev
if errorlevel 1 goto :fail
exit /b 0

:nsis
call pnpm tauri build --bundles nsis
if errorlevel 1 goto :fail
echo.
echo Installer:
for %%f in ("src-tauri\target\release\bundle\nsis\*.exe") do echo   %%~ff
explorer "src-tauri\target\release\bundle\nsis"
pause
exit /b 0

:fail
echo.
echo Failed.
pause
exit /b 1
