@echo off
setlocal enabledelayedexpansion

rem -- extrai a primeira linha que comeca com "version" do Cargo.toml ---------
set "LINE="
for /f "delims=" %%a in ('findstr /b /r /c:"^version[ =]" Cargo.toml') do (
    if not defined LINE set "LINE=%%a"
)

if not defined LINE (
    echo ERRO: nao foi possivel encontrar a linha "version" em Cargo.toml
    exit /b 1
)

rem -- usa as aspas como delimitador: o token 2 e o conteudo entre " e " ------
for /f tokens^=2^ delims^=^" %%v in ("!LINE!") do set "VERSION=%%v"

if not defined VERSION (
    echo ERRO: nao foi possivel parsear a versao em "!LINE!"
    exit /b 1
)

echo Criando tag v!VERSION!
git tag -a v!VERSION! -m "Release v!VERSION!"
if errorlevel 1 exit /b 1

git push origin v!VERSION!
