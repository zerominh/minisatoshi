@echo off
REM Test bare vs braced taproot script leaves (wallet policy TREE grammar).
setlocal
set "LEDGER_DIR=%APPDATA%\com.minisatoshi.desktop\ledger"
set "PY=%LEDGER_DIR%\venv\Scripts\python.exe"
set "REPO=%~dp0.."
copy /Y "%REPO%\tools\_abc_isolate3.py" "%LEDGER_DIR%\_abc_isolate3.py" >nul

echo Approve or reject each prompt on Ledger.
"%PY%" "%LEDGER_DIR%\_abc_isolate3.py"
echo Exit code: %ERRORLEVEL%
endlocal
