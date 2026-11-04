@echo off
rem Release any_txt_parser: fmt check, tests, release build, then publish the exe
rem and HELP.md to dist\ and the bundled skill folders with SHA256 verification.
rem All arguments go to scripts\release.ps1 (Windows PowerShell 5.1), e.g.
rem   scripts\release.bat -DryRun
rem   scripts\release.bat -SkipTests -NoSkills
rem   scripts\release.bat -SkipBuild -ExePath C:\path\any_txt_parser.exe
setlocal
rem Started from a PowerShell 7 session, cmd inherits pwsh's PSModulePath, whose
rem 7.x modules Windows PowerShell 5.1 cannot load; unset, 5.1 uses its default.
set "PSModulePath="
powershell.exe -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "%~dp0release.ps1" %*
exit /b %ERRORLEVEL%
