@echo off
cd /d "C:\Users\Admin\Documents\GitHub\bitcoin-wallet-generator\rust_vanity_generator"
echo Building the program...
cargo build --release
if errorlevel 1 (
    echo Build failed! Press any key to exit...
    pause
    exit /b 1
)

echo.
echo Choose chain to generate addresses for:
echo 1. Bitcoin (BTC)
echo 2. Ethereum/EVM
echo.
set /p choice="Enter your choice (1 or 2): "

if "%choice%"=="1" (
    set chain=btc
) else if "%choice%"=="2" (
    set chain=evm
) else (
    echo Invalid choice. Press any key to exit...
    pause
    exit /b 1
)

echo.
echo Build completed. Press any key to run the program...
pause
echo Running the program...
powershell -Command "Start-Process powershell -Verb RunAs -ArgumentList '-NoExit', '-Command', 'cd \"C:\Users\Admin\Documents\GitHub\bitcoin-wallet-generator\rust_vanity_generator\"; & \"C:\Users\Admin\Documents\GitHub\bitcoin-wallet-generator\rust_vanity_generator\target\release\rust_vanity_generator.exe\" --chain %chain%; pause'" 