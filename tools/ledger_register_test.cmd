@echo off
REM Manual Ledger register_wallet test (same path as Minisatoshi app)
REM Prerequisites: Ledger unlocked, Bitcoin app open (testnet for --chain test)

setlocal
set "LEDGER_DIR=%APPDATA%\com.minisatoshi.desktop\ledger"
set "PY=%LEDGER_DIR%\venv\Scripts\python.exe"
set "CLI=%LEDGER_DIR%\ledger_cli.py"
set "REPO=%~dp0.."
set "JSON=%REPO%\tools\ledger_register_abc_testnet.json"

echo === 1) Runtime check ===
if not exist "%PY%" (
  echo ERROR: venv not found at %PY%
  echo Install via app: Settings -^> Install Ledger signer
  exit /b 1
)
"%PY%" -c "import ledger_bitcoin; import hid; print('ledger-bitcoin', ledger_bitcoin.__version__); print('hid OK')"
if errorlevel 1 exit /b 1

echo.
echo === 2) Refresh ledger_cli.py from repo (optional) ===
copy /Y "%REPO%\tools\ledger_cli.py" "%CLI%" >nul

echo.
echo === 3) Register ABC testnet fixture on device ===
echo JSON: %JSON%
echo Approve prompts on Ledger screen...
type "%JSON%" | "%PY%" "%CLI%" register --chain test
echo.
echo Exit code: %ERRORLEVEL%
endlocal
