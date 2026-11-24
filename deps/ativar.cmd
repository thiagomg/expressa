@echo off
rem Ativa deps\ no prompt do cmd. Na raiz do repositório:
rem   call deps\ativar.cmd
set "DEPS=%~dp0"
set "PATH=%DEPS%gtk\bin;%DEPS%protoc\bin;%PATH%"
set "PKG_CONFIG=%DEPS%gtk\bin\pkgconf.exe"
set "PKG_CONFIG_PATH=%DEPS%gtk\lib\pkgconfig"
set "PROTOC=%DEPS%protoc\bin\protoc.exe"
set "LIB=%DEPS%gtk\lib;%LIB%"
set "INCLUDE=%DEPS%gtk\include;%INCLUDE%"
echo deps ativo: %PKG_CONFIG%
