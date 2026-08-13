@echo off
setlocal
where powershell.exe >nul 2>nul
if errorlevel 1 (
  echo bootstrap: FAILED Windows PowerShell was not found
  exit /b 2
)
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0bootstrap.ps1" %*
exit /b %ERRORLEVEL%
