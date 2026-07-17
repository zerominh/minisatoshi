@echo off
REM Isolate which ABC policy feature Bitcoin Test 2.4.6 rejects.
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
echo === A) wsh + older(1008) + 84'/1'/0' ===
type "%REPO%\tools\ledger_register_wsh_testnet.json" | "%PY%" "%CLI%" register --chain test
echo exit %ERRORLEVEL%

echo.
echo === B) ABC taproot WITHOUT older leaf ===
type "%REPO%\tools\ledger_register_abc_no_older.json" | "%PY%" "%CLI%" register --chain test
echo exit %ERRORLEVEL%

echo.
echo === C) ABC with older(65535) BIP68-max ===
type "%REPO%\tools\ledger_register_abc_older65535.json" | "%PY%" "%CLI%" register --chain test
echo exit %ERRORLEVEL%

echo.
echo === D) Full ABC older(210240) ===
type "%REPO%\tools\ledger_register_abc_testnet.json" | "%PY%" "%CLI%" register --chain test
echo exit %ERRORLEVEL%

endlocal
