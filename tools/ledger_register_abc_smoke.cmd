@echo off
REM ABC-shaped taproot register using THIS Ledger as key A.
REM Close Ledger Live. Unlock. Open Bitcoin Test.

setlocal
set "LEDGER_DIR=%APPDATA%\com.minisatoshi.desktop\ledger"
set "PY=%LEDGER_DIR%\venv\Scripts\python.exe"
set "CLI=%LEDGER_DIR%\ledger_cli.py"
set "REPO=%~dp0.."
copy /Y "%REPO%\tools\ledger_cli.py" "%CLI%" >nul

echo === Probe ===
"%PY%" "%CLI%" probe --chain test
if errorlevel 1 exit /b 1

echo.
echo === A) ABC taproot WITHOUT older — approve on device ===
"%PY%" "%CLI%" abc-smoke --chain test
echo exit %ERRORLEVEL%

echo.
echo === B) ABC with older(65535) BIP68-max — approve on device ===
"%PY%" "%CLI%" abc-smoke --chain test --with-older
echo exit %ERRORLEVEL%
endlocal
