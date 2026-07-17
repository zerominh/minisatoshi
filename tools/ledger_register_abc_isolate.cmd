@echo off
REM Step through taproot policies until one is accepted (identify 0x6a82 cause).
REM Close Ledger Live. Unlock. Open Bitcoin Test. Approve each prompt (or reject).

setlocal
set "LEDGER_DIR=%APPDATA%\com.minisatoshi.desktop\ledger"
set "PY=%LEDGER_DIR%\venv\Scripts\python.exe"
set "REPO=%~dp0.."
copy /Y "%REPO%\tools\ledger_cli.py" "%LEDGER_DIR%\ledger_cli.py" >nul
copy /Y "%REPO%\tools\_abc_isolate.py" "%LEDGER_DIR%\_abc_isolate.py" >nul

echo Each case may show Approve on Ledger — accept or reject, then next runs.
"%PY%" "%LEDGER_DIR%\_abc_isolate.py"
echo.
echo Exit code: %ERRORLEVEL%
endlocal
