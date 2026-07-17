@echo off
REM Taproot script-tree retest: keys ONLY from this Ledger (correct derivation paths).
setlocal
set "LEDGER_DIR=%APPDATA%\com.minisatoshi.desktop\ledger"
set "PY=%LEDGER_DIR%\venv\Scripts\python.exe"
set "REPO=%~dp0.."
copy /Y "%REPO%\tools\_abc_isolate2.py" "%LEDGER_DIR%\_abc_isolate2.py" >nul

echo Approve or reject each prompt on Ledger.
"%PY%" "%LEDGER_DIR%\_abc_isolate2.py"
echo Exit code: %ERRORLEVEL%
endlocal
