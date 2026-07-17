@echo off
REM Ledger policy smoke tests — find what your device accepts.
REM Unlock Ledger, open Bitcoin app (testnet for --chain test).

setlocal
set "LEDGER_DIR=%APPDATA%\com.minisatoshi.desktop\ledger"
set "PY=%LEDGER_DIR%\venv\Scripts\python.exe"
set "CLI=%LEDGER_DIR%\ledger_cli.py"
set "REPO=%~dp0.."

if not exist "%PY%" (
  echo venv missing: %PY%
  exit /b 1
)

echo === A) BIP-86 single-key tr(@0/**) — baseline (should work on app ^>= 2.2.1) ===
echo Replace tpub below with YOUR Ledger xpub from Settings -^> Get xpub (86'/1'/0' testnet).
echo Skipping if you have not set MY_TPUB in env.
if defined MY_TPUB (
  echo {"name":"BIP86 test","policy":"tr(@0/**)","keys":["[a98a1256/86'/1'/0']%MY_TPUB%"]} | "%PY%" "%CLI%" register --chain test
  echo exit %ERRORLEVEL%
) else (
  echo Set MY_TPUB=your_tpub then re-run this block.
)

echo.
echo === B) Liana-style wsh miniscript (SegWit, not Taproot ABC) ===
type "%REPO%\tools\ledger_register_wsh_testnet.json" | "%PY%" "%CLI%" register --chain test
echo exit %ERRORLEVEL%

echo.
echo === C) ABC tr(NUMS, binary taptree) — Minisatoshi ABC (often rejected 0x6a80) ===
type "%REPO%\tools\ledger_register_abc_testnet.json" | "%PY%" "%CLI%" register --chain test
echo exit %ERRORLEVEL%

endlocal
