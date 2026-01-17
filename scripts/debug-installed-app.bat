@echo off
echo ================================================
echo Soul Player Debug Launcher
echo ================================================
echo.

set RUST_BACKTRACE=full
set RUST_LOG=debug

echo Starting app with debug logging...
echo If the app crashes, you'll see the error below.
echo.

"C:\Program Files\Soul Player\soul-player.exe"

echo.
echo ================================================
echo App exited with code: %ERRORLEVEL%
echo ================================================
echo.
pause
