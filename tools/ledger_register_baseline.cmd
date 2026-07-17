@echo off
REM Baseline REGISTER_WALLET using THIS Ledger's fingerprint + xpub.
REM Close Ledger Live. Unlock. Open Bitcoin Test. Approve on device.

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
echo === Baseline: wsh(sortedmulti) with YOUR device key + 1 external ===
echo Previous fixture-only keys failed because none matched this Ledger.
echo Approve the policy on the device screen...
"%PY%" "%CLI%" baseline --chain test
echo.
echo Exit code: %ERRORLEVEL%
endlocal
