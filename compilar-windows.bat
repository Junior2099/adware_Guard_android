@echo off
setlocal
cd /d "%~dp0"
cargo check
if errorlevel 1 goto erro
cargo test
if errorlevel 1 goto erro
cargo build --release
if errorlevel 1 goto erro
if not exist dist mkdir dist
copy /Y target\release\adwareguard.exe dist\
xcopy /E /I /Y data dist\data
xcopy /E /I /Y tools dist\tools
copy /Y README.md dist\
echo Pronto. Execute dist\adwareguard.exe. Mantenha data e tools junto do executavel.
pause
exit /b 0
:erro
echo Compilacao falhou. Leia a primeira mensagem de erro acima.
pause
exit /b 1
