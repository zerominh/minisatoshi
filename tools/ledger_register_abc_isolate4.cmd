@echo off
setlocal
set "LEDGER_DIR=%APPDATA%\com.minisatoshi.desktop\ledger"
set "PY=%LEDGER_DIR%\venv\Scripts\python.exe"
set "REPO=%~dp0.."
copy /Y "%REPO%\tools\_abc_isolate4.py" "%LEDGER_DIR%\_abc_isolate4.py" >nul
"%PY%" "%LEDGER_DIR%\_abc_isolate4.py"
echo Exit code: %ERRORLEVEL%
endlocal
