@echo off
node "%~dp0rustc-build-script-wrapper.mjs" %*
exit /b %errorlevel%
