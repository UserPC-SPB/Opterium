@echo off
rem A release build is the full release pipeline (scripts\release.ps1): fmt check,
rem tests, cargo build --release --locked, then the exe and HELP.md are copied to
rem dist\ and the bundled skills and every copy is SHA256-verified, so the shipped
rem binaries cannot drift from target\release. Arguments are passed through, e.g.
rem   build_release.bat -DryRun        build_release.bat -SkipTests
rem A bare compile without publishing:  cargo build --release --locked
call "%~dp0scripts\release.bat" %*
exit /b %ERRORLEVEL%
