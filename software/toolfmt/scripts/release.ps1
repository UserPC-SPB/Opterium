#Requires -Version 5.0
<#
.SYNOPSIS
    Build any_txt_parser and publish the exe and HELP.md to every release
    location, so the shipped binaries and docs never drift from the sources.

.DESCRIPTION
    One release path, fail-fast (exit code 1 on the first error):

      1. Version gate: src\lib.rs PARSER_VERSION == Cargo.toml [package]
         version == Cargo.lock any_txt_parser version, and the HELP.md title
         names the same version.
      2. Layout check: every bundled skill folder that exists must have a
         SKILL.md with an "any_txt_parser X.Y.Z" mention. Missing skill
         folders are skipped with a warning. Every existing destination must
         be writable: a running skill exe or a file another program holds
         open stops the release here, and again right before step 7.
      3. cargo fmt --check
      4. cargo test --locked [--offline]               (-SkipTests skips it)
      5. cargo build --release --locked [--offline]    (-SkipBuild skips it)
         with the dependency paths under CARGO_HOME remapped, so the exe
         names no folder of the build machine.
      6. "<exe> --version" must print "any_txt_parser <version>".
      7. Copy the exe and HELP.md to dist\ and to each skill folder
         (bin\ or assets\ for the exe, references\parser.md for HELP.md).
      8. Rewrite every "any_txt_parser X.Y.Z" mention in each SKILL.md to
         the release version (encoding, BOM and line endings are kept).
      9. Verify the SHA256 of every copy and print a manifest.

    Steps 1, 2 and (with -SkipBuild) 6 run before anything is written.
    When <repo>\target\cargo-home exists it becomes CARGO_HOME and cargo runs
    with --offline. The MSVC environment comes from vcvars64.bat of the newest
    Visual Studio with the x64 C++ tools (vswhere -latest; Build Tools 2022
    path as a fallback), unless the release starts in an already initialized
    environment (VCINSTALLDIR set, e.g. a VS x64 Native Tools prompt), which is
    then used as is. The plan and the manifest name the MSVC toolset.
    Windows PowerShell 5.1 and pwsh 7 both work, also when release.bat is
    started from a PowerShell 7 session.

.PARAMETER SkipTests
    Do not run "cargo test".

.PARAMETER SkipBuild
    Do not run "cargo build"; publish a prebuilt exe instead (default
    target\release\any_txt_parser.exe, or -ExePath). It must still report
    the release version.

.PARAMETER ExePath
    Prebuilt exe to publish. Only valid together with -SkipBuild.

.PARAMETER DryRun
    Run the read-only checks and print the plan. Nothing is built, copied or
    modified.

.PARAMETER NoSkills
    Publish to dist\ only; leave the skill folders untouched.

.PARAMETER RepoRoot
    Repository to release. Defaults to the parent of this script's folder.

.EXAMPLE
    scripts\release.bat

.EXAMPLE
    scripts\release.bat -DryRun

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\release.ps1 -SkipTests -NoSkills
#>
[CmdletBinding()]
param(
    [switch]$SkipTests,
    [switch]$SkipBuild,
    [string]$ExePath = '',
    [switch]$DryRun,
    [switch]$NoSkills,
    [string]$RepoRoot = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$ExeName = 'any_txt_parser.exe'
$HelpName = 'HELP.md'
$SkillDocName = 'references\parser.md'
# Bundled skill folders (relative to the repository) and where each keeps the exe.
$SkillLayouts = @(
    [pscustomobject]@{ Root = '.dsh\skills\any-txt-parser'; Exe = 'bin\any_txt_parser.exe' },
    [pscustomobject]@{ Root = '.pi\skills\any-txt-parser'; Exe = 'bin\any_txt_parser.exe' },
    [pscustomobject]@{ Root = '.hermes\skills\any-txt-parser'; Exe = 'assets\any_txt_parser.exe' }
)
$VcVarsFallback = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
$SemVerCore = '\d+\.\d+\.\d+(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?'
$SemVer = '^' + $SemVerCore + '$'
# "any_txt_parser 1.0.3" or "any_txt_parser v1.0.3"; the version is replaced, the prefix kept.
$VersionMention = '(?<prefix>\bany_txt_parser v?)(?<version>' + $SemVerCore + ')(?![.-]?[0-9A-Za-z])'

$script:Repo = ''
$script:WarningCount = 0

function Fail([string]$Message) {
    throw $Message
}

function Write-Info([string]$Text) {
    [Console]::Out.WriteLine('[release] ' + $Text)
}

function Write-Warn([string]$Text) {
    $script:WarningCount++
    [Console]::Error.WriteLine('[release] WARNING: ' + $Text)
}

function Get-RelativePath([string]$Path) {
    $root = $script:Repo.TrimEnd('\') + '\'
    if ($Path.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $Path.Substring($root.Length)
    }
    return $Path
}

# Plain .NET instead of Get-FileHash: in Windows PowerShell 5.1 that is a
# function of the auto-loaded Microsoft.PowerShell.Utility module, and a
# PSModulePath inherited from a PowerShell 7 session (pwsh -> cmd -> powershell)
# makes the auto-load pick the 7.x module, which 5.1 cannot load.
function Get-Sha256([string]$Path) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $stream = [System.IO.File]::OpenRead($Path)
        try {
            $hash = $sha.ComputeHash($stream)
        } finally {
            $stream.Dispose()
        }
    } finally {
        $sha.Dispose()
    }
    return ([System.BitConverter]::ToString($hash) -replace '-', '').ToLowerInvariant()
}

function Read-Utf8File([string]$Path) {
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    $bom = $bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF
    $offset = 0
    if ($bom) { $offset = 3 }
    $strict = New-Object System.Text.UTF8Encoding($false, $true)
    try {
        $text = $strict.GetString($bytes, $offset, $bytes.Length - $offset)
    } catch {
        Fail ("{0} is not valid UTF-8" -f (Get-RelativePath $Path))
    }
    return [pscustomobject]@{ Text = $text; Bom = $bom }
}

function Write-Utf8File([string]$Path, [string]$Text, [bool]$Bom) {
    [System.IO.File]::WriteAllText($Path, $Text, (New-Object System.Text.UTF8Encoding($Bom)))
}

function Get-MentionedVersions([string]$Text) {
    $versions = @()
    foreach ($match in [regex]::Matches($Text, $VersionMention)) {
        $versions += $match.Groups['version'].Value
    }
    return , $versions
}

function Join-Arguments([string[]]$Arguments) {
    $quoted = foreach ($argument in $Arguments) {
        if ($argument.Contains('"')) { Fail "unsupported quote in argument: $argument" }
        if ($argument -eq '' -or $argument -match '\s') { '"' + $argument + '"' } else { $argument }
    }
    return ($quoted -join ' ')
}

# Runs a native program with inherited console handles (no PowerShell stream
# conversion of stderr, so cargo progress output never becomes an error record).
function Invoke-Native([string]$FilePath, [string[]]$Arguments, [switch]$CaptureStdout) {
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $FilePath
    $info.Arguments = Join-Arguments $Arguments
    $info.WorkingDirectory = $script:Repo
    $info.UseShellExecute = $false
    if ($CaptureStdout) {
        $info.RedirectStandardOutput = $true
        $info.StandardOutputEncoding = New-Object System.Text.UTF8Encoding($false)
    }
    $process = [System.Diagnostics.Process]::Start($info)
    $stdout = ''
    if ($CaptureStdout) { $stdout = $process.StandardOutput.ReadToEnd() }
    $process.WaitForExit()
    return [pscustomobject]@{ ExitCode = $process.ExitCode; Stdout = $stdout }
}

function Resolve-Repository([string]$Requested) {
    $candidate = $Requested
    if (-not $candidate) { $candidate = Split-Path -Parent $PSScriptRoot }
    if (-not (Test-Path -LiteralPath $candidate -PathType Container)) {
        Fail "repository folder not found: $candidate"
    }
    $root = (Resolve-Path -LiteralPath $candidate).ProviderPath.TrimEnd('\')
    foreach ($required in @('Cargo.toml', 'Cargo.lock', 'src\lib.rs', $HelpName)) {
        if (-not (Test-Path -LiteralPath (Join-Path $root $required) -PathType Leaf)) {
            Fail "$required not found in $root (is -RepoRoot the any_txt_parser repository?)"
        }
    }
    return $root
}

function Get-ReleaseVersion {
    $lib = (Read-Utf8File (Join-Path $script:Repo 'src\lib.rs')).Text
    $match = [regex]::Match($lib, '(?m)^\s*pub\s+const\s+PARSER_VERSION\s*:\s*&(?:''static\s+)?str\s*=\s*"(?<v>[^"]*)"\s*;')
    if (-not $match.Success) { Fail 'src\lib.rs: "pub const PARSER_VERSION: &str = ..." not found' }
    $parserVersion = $match.Groups['v'].Value

    $packageVersion = $null
    $section = ''
    foreach ($line in [System.IO.File]::ReadAllLines((Join-Path $script:Repo 'Cargo.toml'))) {
        $trimmed = $line.Trim()
        if ($trimmed -match '^\[\[?\s*([^\]]+?)\s*\]') {
            $section = $Matches[1]
            continue
        }
        if ($section -eq 'package' -and $trimmed -match '^version\s*=\s*"([^"]*)"') {
            $packageVersion = $Matches[1]
            break
        }
    }
    if ($null -eq $packageVersion) { Fail 'Cargo.toml: [package] version = "..." not found' }

    $lock = [System.IO.File]::ReadAllText((Join-Path $script:Repo 'Cargo.lock'))
    $match = [regex]::Match($lock, '(?m)^name = "any_txt_parser"\r?\nversion = "(?<v>[^"]*)"')
    if (-not $match.Success) { Fail 'Cargo.lock: package any_txt_parser not found' }
    $lockVersion = $match.Groups['v'].Value

    if ($parserVersion -cne $packageVersion) {
        Fail "version mismatch: src\lib.rs PARSER_VERSION = $parserVersion, Cargo.toml [package] version = $packageVersion"
    }
    if ($lockVersion -cne $packageVersion) {
        Fail "version mismatch: Cargo.lock any_txt_parser = $lockVersion, Cargo.toml [package] version = $packageVersion (refresh Cargo.lock with a build without --locked)"
    }
    if ($parserVersion -notmatch $SemVer) { Fail "PARSER_VERSION '$parserVersion' is not a semantic version" }

    $helpTitle = $null
    foreach ($line in ((Read-Utf8File (Join-Path $script:Repo $HelpName)).Text -split "\r?\n")) {
        if ($line -match '^#\s') { $helpTitle = $line; break }
    }
    if ($null -eq $helpTitle) { Fail "$HelpName has no '# ' title line" }
    $titleVersions = Get-MentionedVersions $helpTitle
    if ($titleVersions.Count -eq 0) { Fail "$HelpName title does not name the version: $helpTitle" }
    foreach ($titleVersion in $titleVersions) {
        if ($titleVersion -cne $parserVersion) {
            Fail "$HelpName title names v$titleVersion but the release version is $parserVersion - update $HelpName first"
        }
    }
    return $parserVersion
}

function Get-TargetDirectory {
    $dir = $env:CARGO_TARGET_DIR
    if (-not $dir) { return (Join-Path $script:Repo 'target') }
    if (-not [System.IO.Path]::IsPathRooted($dir)) { $dir = Join-Path $script:Repo $dir }
    return $dir
}

function Get-ReleasePlan([string]$Version) {
    $copies = New-Object System.Collections.Generic.List[object]
    $skillFiles = New-Object System.Collections.Generic.List[object]
    $skipped = New-Object System.Collections.Generic.List[string]
    $help = Join-Path $script:Repo $HelpName
    $copies.Add([pscustomobject]@{ Kind = 'exe'; Destination = (Join-Path $script:Repo ('dist\' + $ExeName)) })
    $copies.Add([pscustomobject]@{ Kind = 'doc'; Destination = (Join-Path $script:Repo ('dist\' + $HelpName)) })
    # The README travels with the exe: an AI given dist\ reads it first.
    if (Test-Path -LiteralPath (Join-Path $script:Repo 'README.md') -PathType Leaf) {
        $copies.Add([pscustomobject]@{ Kind = 'readme'; Destination = (Join-Path $script:Repo 'dist\README.md') })
    }
    if (-not $NoSkills) {
        foreach ($layout in $SkillLayouts) {
            $root = Join-Path $script:Repo $layout.Root
            if (-not (Test-Path -LiteralPath $root -PathType Container)) {
                $skipped.Add($layout.Root)
                Write-Warn "skill folder missing, skipped: $($layout.Root)"
                continue
            }
            $skillMd = Join-Path $root 'SKILL.md'
            if (-not (Test-Path -LiteralPath $skillMd -PathType Leaf)) {
                Fail "$(Get-RelativePath $skillMd) not found (the skill folder exists, so its SKILL.md must too)"
            }
            $mentioned = Get-MentionedVersions (Read-Utf8File $skillMd).Text
            if ($mentioned.Count -eq 0) {
                Fail "$(Get-RelativePath $skillMd) has no 'any_txt_parser X.Y.Z' version mention to update"
            }
            $copies.Add([pscustomobject]@{ Kind = 'exe'; Destination = (Join-Path $root $layout.Exe) })
            $copies.Add([pscustomobject]@{ Kind = 'doc'; Destination = (Join-Path $root $SkillDocName) })
            $stale = @($mentioned | Where-Object { $_ -cne $Version })
            $skillFiles.Add([pscustomobject]@{ Path = $skillMd; Before = (($mentioned | Select-Object -Unique) -join ', '); NeedsUpdate = ($stale.Count -gt 0) })
        }
    }
    return [pscustomobject]@{ Help = $help; Copies = $copies; SkillFiles = $skillFiles; Skipped = $skipped }
}

# Opens every existing destination the way the publishing step will write it
# and closes it again without writing (contents and timestamps stay as they
# are). A running skill exe, a file another program holds without write
# sharing, or a SKILL.md this account may not write then stops the release
# before the first copy instead of leaving some destinations updated and the
# rest stale. Copy-Item tolerates holders that share writing and replaces
# read-only files (-Force), so read-only copies are only checked for
# exclusive locks; WriteAllText (SKILL.md) allows other readers only.
function Assert-DestinationsWritable($Plan) {
    $targets = New-Object System.Collections.Generic.List[object]
    foreach ($copy in $Plan.Copies) {
        $targets.Add([pscustomobject]@{ Path = $copy.Destination; Share = [System.IO.FileShare]::ReadWrite; ReadOnlyOk = $true })
    }
    foreach ($skill in $Plan.SkillFiles) {
        if ($skill.NeedsUpdate) {
            $targets.Add([pscustomobject]@{ Path = $skill.Path; Share = [System.IO.FileShare]::Read; ReadOnlyOk = $false })
        }
    }
    $blocked = New-Object System.Collections.Generic.List[string]
    foreach ($target in $targets) {
        if (-not (Test-Path -LiteralPath $target.Path -PathType Leaf)) { continue }
        $access = [System.IO.FileAccess]::Write
        if ($target.ReadOnlyOk -and (Get-Item -LiteralPath $target.Path -Force).IsReadOnly) { $access = [System.IO.FileAccess]::Read }
        try {
            [System.IO.File]::Open($target.Path, [System.IO.FileMode]::Open, $access, $target.Share).Dispose()
        } catch {
            $blocked.Add(('{0}: {1}' -f (Get-RelativePath $target.Path), $_.Exception.GetBaseException().Message.Trim()))
        }
    }
    if ($blocked.Count -gt 0) {
        Fail ("{0} destination(s) cannot be written, nothing was published; close the program using them and rerun: {1}" -f $blocked.Count, ($blocked -join ' | '))
    }
}

function Find-VcVars {
    $programFilesX86 = [Environment]::GetFolderPath('ProgramFilesX86')
    if ($programFilesX86) {
        $vswhere = Join-Path $programFilesX86 'Microsoft Visual Studio\Installer\vswhere.exe'
        if (Test-Path -LiteralPath $vswhere -PathType Leaf) {
            $found = Invoke-Native $vswhere @('-latest', '-utf8', '-products', '*', '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-property', 'installationPath') -CaptureStdout
            if ($found.ExitCode -eq 0) {
                foreach ($installation in ($found.Stdout -split "\r?\n")) {
                    if (-not $installation.Trim()) { continue }
                    $candidate = Join-Path $installation.Trim() 'VC\Auxiliary\Build\vcvars64.bat'
                    if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
                }
            }
        }
    }
    if (Test-Path -LiteralPath $VcVarsFallback -PathType Leaf) { return $VcVarsFallback }
    return $null
}

# Imports the environment produced by vcvars64.bat into this process. The batch
# path travels through an environment variable and "set" output is read as
# UTF-16 (cmd /u), so non-ASCII profile paths survive.
function Import-VcVars([string]$VcVars) {
    $helper = Join-Path ([System.IO.Path]::GetTempPath()) ("atp-release-vcvars-{0}.cmd" -f $PID)
    $lines = "@echo off`r`ncall `"%ATP_RELEASE_VCVARS%`" >nul 2>&1`r`nset`r`n"
    [System.IO.File]::WriteAllText($helper, $lines, [System.Text.Encoding]::ASCII)
    try {
        $env:ATP_RELEASE_VCVARS = $VcVars
        $info = New-Object System.Diagnostics.ProcessStartInfo
        $info.FileName = Join-Path ([Environment]::SystemDirectory) 'cmd.exe'
        # /s: strip only the outer quote pair, so any temp path survives.
        $info.Arguments = '/d /u /s /c ""' + $helper + '""'
        $info.UseShellExecute = $false
        $info.RedirectStandardOutput = $true
        $info.StandardOutputEncoding = [System.Text.Encoding]::Unicode
        $process = [System.Diagnostics.Process]::Start($info)
        $dump = $process.StandardOutput.ReadToEnd()
        $process.WaitForExit()
    } finally {
        Remove-Item -LiteralPath $helper -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath 'Env:ATP_RELEASE_VCVARS' -ErrorAction SilentlyContinue
    }
    $imported = @{}
    foreach ($line in ($dump -split "\r?\n")) {
        $equals = $line.IndexOf('=')
        if ($equals -gt 0) { $imported[$line.Substring(0, $equals)] = $line.Substring($equals + 1) }
    }
    if ($process.ExitCode -ne 0 -or -not $imported.ContainsKey('VCINSTALLDIR')) {
        Fail "vcvars64.bat did not set up the MSVC environment: $VcVars"
    }
    foreach ($name in $imported.Keys) {
        if ($name -ne 'ATP_RELEASE_VCVARS') {
            [Environment]::SetEnvironmentVariable($name, $imported[$name], 'Process')
        }
    }
}

function Find-Cargo {
    $command = Get-Command 'cargo.exe' -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) { return $command.Path }
    $candidate = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
    if (Test-Path -LiteralPath $candidate -PathType Leaf) {
        $env:PATH = (Split-Path -Parent $candidate) + ';' + $env:PATH
        return $candidate
    }
    Fail 'cargo.exe not found on PATH or in %USERPROFILE%\.cargo\bin'
}

# Decides, without changing anything, which MSVC environment cargo will run in.
# vswhere -latest picks the newest Visual Studio with the x64 C++ tools: with
# VS 18 and VS 2022 installed that is VS 18, whose linker and static CRT then go
# into the exe. To build with another installation, start the release from its
# x64 Native Tools prompt; an already active VCINSTALLDIR is used as is. The
# plan and the manifest name the toolset either way.
function Resolve-Msvc([bool]$NeedMsvc) {
    $kind = 'missing'
    $vcvars = ''
    if (-not $NeedMsvc) {
        $kind = 'unused'
    } elseif ($env:VCINSTALLDIR) {
        $kind = 'active'
    } else {
        $found = Find-VcVars
        if ($found) {
            $kind = 'vcvars'
            $vcvars = $found
        }
    }
    return [pscustomobject]@{ Kind = $kind; VcVars = $vcvars }
}

function Format-Msvc($Msvc) {
    $toolset = 'MSVC'
    # VCToolsVersion is only meaningful once the environment is active.
    if ($env:VCINSTALLDIR -and $env:VCToolsVersion) { $toolset = 'MSVC ' + $env:VCToolsVersion.Trim().TrimEnd('\') }
    switch ($Msvc.Kind) {
        'unused' { return 'not needed (-SkipTests -SkipBuild)' }
        'active' { return ('{0} from the calling environment ({1})' -f $toolset, $env:VCINSTALLDIR.TrimEnd('\')) }
        'vcvars' { return ('{0} via {1}' -f $toolset, $Msvc.VcVars) }
    }
    return "vcvars64.bat not found (vswhere, $VcVarsFallback); rustc's own MSVC detection"
}

function Initialize-Toolchain($Msvc) {
    $cargoHome = Join-Path $script:Repo 'target\cargo-home'
    if (Test-Path -LiteralPath $cargoHome -PathType Container) {
        $env:CARGO_HOME = $cargoHome
        Write-Info "CARGO_HOME = $cargoHome (offline build)"
    }
    switch ($Msvc.Kind) {
        'active' { Write-Info "MSVC environment already active: $env:VCINSTALLDIR" }
        'vcvars' {
            Write-Info "MSVC environment from $($Msvc.VcVars)"
            Import-VcVars $Msvc.VcVars
        }
        'missing' { Write-Warn "vcvars64.bat not found (vswhere and $VcVarsFallback); relying on rustc's own MSVC detection" }
    }
    return (Find-Cargo)
}

function Invoke-Cargo([string]$Cargo, [string[]]$Arguments) {
    Write-Info ('==> cargo ' + ($Arguments -join ' '))
    $result = Invoke-Native $Cargo $Arguments
    if ($result.ExitCode -ne 0) {
        Fail ('cargo {0} failed with exit code {1}' -f ($Arguments -join ' '), $result.ExitCode)
    }
}

# Panic locations of dependencies hold absolute paths under
# <CARGO_HOME>\registry\src; remapped, the exe names no folder of the build
# machine and builds from different checkouts or CARGO_HOMEs match. The array
# is appended to the rustflags of .cargo\config.toml.
function Get-RemapConfig {
    $cargoHome = $env:CARGO_HOME
    if (-not $cargoHome) { $cargoHome = Join-Path $env:USERPROFILE '.cargo' }
    $registry = Join-Path ([System.IO.Path]::GetFullPath($cargoHome)) 'registry\src'
    if ($registry.Contains("'")) { Fail "CARGO_HOME with a quote is not supported: $cargoHome" }
    return "target.x86_64-pc-windows-msvc.rustflags=['--remap-path-prefix=$registry=cargo-registry']"
}

function Build-Release([string]$Cargo, [bool]$Offline) {
    $arguments = @('build', '--release', '--locked')
    if ($Offline) { $arguments += '--offline' }
    $arguments += @('--config', (Get-RemapConfig))
    $arguments += '--message-format=json-render-diagnostics'
    Write-Info ('==> cargo ' + ($arguments -join ' '))
    $result = Invoke-Native $Cargo $arguments -CaptureStdout
    if ($result.ExitCode -ne 0) { Fail "cargo build --release failed with exit code $($result.ExitCode)" }
    # Ask cargo which file it produced instead of guessing the target directory.
    $built = $null
    foreach ($line in ($result.Stdout -split "\r?\n")) {
        if (-not $line.StartsWith('{')) { continue }
        $message = $line | ConvertFrom-Json
        if ($message.reason -ne 'compiler-artifact') { continue }
        if ($message.target.name -eq 'any_txt_parser' -and @($message.target.kind) -contains 'bin' -and $message.executable) {
            $built = [string]$message.executable
        }
    }
    if (-not $built) { Fail 'cargo build did not report the any_txt_parser executable' }
    return $built
}

function Assert-ExeVersion([string]$Exe, [string]$Version) {
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $Exe
    $info.Arguments = '--version'
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    try {
        $process = [System.Diagnostics.Process]::Start($info)
    } catch {
        Fail "cannot start $Exe : $($_.Exception.Message)"
    }
    $process.StandardInput.Close()
    $stdout = $process.StandardOutput.ReadToEndAsync()
    # Generous: the first start of a freshly built exe can wait for an
    # antivirus scan, and a busy disk slows it further.
    if (-not $process.WaitForExit(180000)) {
        try { $process.Kill() } catch { }
        Fail "$Exe --version did not finish within 180 s"
    }
    $process.WaitForExit()
    $printed = $stdout.Result.Trim()
    $expected = "any_txt_parser $Version"
    if ($process.ExitCode -ne 0) { Fail "$Exe --version exited with code $($process.ExitCode)" }
    if ($printed -cne $expected) {
        Fail "$Exe --version printed '$printed', expected '$expected' (stale or foreign binary)"
    }
}

function Assert-PrebuiltIsFresh([string]$Exe) {
    $exeTime = (Get-Item -LiteralPath $Exe).LastWriteTimeUtc
    $inputs = @(Get-ChildItem -LiteralPath (Join-Path $script:Repo 'src') -Recurse -File)
    foreach ($name in @('Cargo.toml', 'Cargo.lock', '.cargo\config.toml')) {
        $path = Join-Path $script:Repo $name
        if (Test-Path -LiteralPath $path -PathType Leaf) { $inputs += Get-Item -LiteralPath $path }
    }
    $newer = @($inputs | Where-Object { $_.LastWriteTimeUtc -gt $exeTime } | Sort-Object LastWriteTimeUtc -Descending)
    if ($newer.Count -gt 0) {
        Write-Warn ("prebuilt exe is older than {0} source file(s), newest: {1}" -f $newer.Count, (Get-RelativePath $newer[0].FullName))
    }
}

function Show-Plan($Plan, [string]$Version, [string]$Exe, [bool]$Offline, $Msvc) {
    $offlineFlag = ''
    if ($Offline) { $offlineFlag = ' --offline' }
    Write-Info "repository : $script:Repo"
    Write-Info "version    : $Version (src\lib.rs = Cargo.toml = Cargo.lock = $HelpName title)"
    Write-Info "msvc       : $(Format-Msvc $Msvc)"
    Write-Info 'plan:'
    Write-Info '  1. cargo fmt --check'
    if ($SkipTests) { Write-Info '  2. cargo test: skipped (-SkipTests)' }
    else { Write-Info "  2. cargo test --locked$offlineFlag" }
    if ($SkipBuild) { Write-Info "  3. cargo build: skipped (-SkipBuild), prebuilt $(Get-RelativePath $Exe)" }
    else { Write-Info "  3. cargo build --release --locked$offlineFlag" }
    Write-Info "  4. exe --version must print 'any_txt_parser $Version'"
    Write-Info "  5. copy exe and $HelpName to:"
    foreach ($copy in $Plan.Copies) { Write-Info ('       {0}  {1}' -f $copy.Kind, (Get-RelativePath $copy.Destination)) }
    Write-Info "  6. set 'any_txt_parser $Version' in:"
    if ($NoSkills) { Write-Info '       (none: -NoSkills)' }
    foreach ($skill in $Plan.SkillFiles) {
        Write-Info ('       {0}  (now: {1})' -f (Get-RelativePath $skill.Path), $skill.Before)
    }
    foreach ($skipped in $Plan.Skipped) { Write-Info "       skipped: $skipped (folder missing)" }
    Write-Info '  7. verify SHA256 of every copy, print the manifest'
}

function Invoke-Release {
    if ($ExePath -and -not $SkipBuild) { Fail '-ExePath requires -SkipBuild (a fresh build always publishes its own output)' }
    $script:Repo = Resolve-Repository $RepoRoot
    $version = Get-ReleaseVersion
    $plan = Get-ReleasePlan $version
    Assert-DestinationsWritable $plan
    $offline = Test-Path -LiteralPath (Join-Path $script:Repo 'target\cargo-home') -PathType Container
    $msvc = Resolve-Msvc (-not ($SkipTests -and $SkipBuild))

    $exe = Join-Path (Join-Path (Get-TargetDirectory) 'release') $ExeName
    if ($SkipBuild) {
        if ($ExePath) {
            if (-not (Test-Path -LiteralPath $ExePath -PathType Leaf)) { Fail "-ExePath not found: $ExePath" }
            $exe = (Resolve-Path -LiteralPath $ExePath).ProviderPath
        }
        if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { Fail "prebuilt exe not found: $exe (build it or pass -ExePath)" }
        Assert-ExeVersion $exe $version
        Assert-PrebuiltIsFresh $exe
    }

    Show-Plan $plan $version $exe $offline $msvc
    if ($DryRun) {
        Write-Info 'DRY RUN: nothing was built, copied or modified.'
        return
    }

    $cargo = Initialize-Toolchain $msvc
    Invoke-Cargo $cargo @('fmt', '--check')
    $tests = 'skipped (-SkipTests)'
    if (-not $SkipTests) {
        $arguments = @('test', '--locked')
        if ($offline) { $arguments += '--offline' }
        Invoke-Cargo $cargo $arguments
        $tests = 'passed'
    }
    $build = 'prebuilt, -SkipBuild'
    if (-not $SkipBuild) {
        $exe = Build-Release $cargo $offline
        Assert-ExeVersion $exe $version
        $build = 'built by this run'
    }

    $exeHash = Get-Sha256 $exe
    $helpHash = Get-Sha256 $plan.Help
    # Again right before the first write: the tests and the build take minutes.
    Assert-DestinationsWritable $plan
    $results = New-Object System.Collections.Generic.List[object]
    $skillResults = New-Object System.Collections.Generic.List[object]
    $written = New-Object System.Collections.Generic.List[string]
    try {
        foreach ($copy in $plan.Copies) {
            $source = $plan.Help
            $expected = $helpHash
            if ($copy.Kind -eq 'exe') {
                $source = $exe
                $expected = $exeHash
            } elseif ($copy.Kind -eq 'readme') {
                $source = Join-Path $script:Repo 'README.md'
                $expected = Get-Sha256 $source
            }
            $state = 'new'
            if (Test-Path -LiteralPath $copy.Destination -PathType Leaf) {
                $state = 'updated'
                if ((Get-Sha256 $copy.Destination) -eq $expected) { $state = 'unchanged' }
            }
            $sameFile = [string]::Equals([System.IO.Path]::GetFullPath($source), [System.IO.Path]::GetFullPath($copy.Destination), [System.StringComparison]::OrdinalIgnoreCase)
            if (-not $sameFile) {
                [void][System.IO.Directory]::CreateDirectory((Split-Path -Parent $copy.Destination))
                Copy-Item -LiteralPath $source -Destination $copy.Destination -Force
                $written.Add((Get-RelativePath $copy.Destination))
            }
            $results.Add([pscustomobject]@{ Path = $copy.Destination; Expected = $expected; State = $state })
        }

        foreach ($skill in $plan.SkillFiles) {
            $file = Read-Utf8File $skill.Path
            $updated = [regex]::Replace($file.Text, $VersionMention, '${prefix}' + $version)
            $state = 'unchanged'
            if ($updated -cne $file.Text) {
                Write-Utf8File $skill.Path $updated $file.Bom
                $written.Add((Get-RelativePath $skill.Path))
                $state = 'updated'
            }
            $skillResults.Add([pscustomobject]@{ Path = $skill.Path; State = $state; Before = $skill.Before })
        }
    } catch {
        $done = 'nothing'
        if ($written.Count -gt 0) { $done = $written -join ', ' }
        Fail ("publishing stopped part-way: {0} Already written: {1}. The other destinations are stale; fix the cause and rerun the release (it rewrites every destination)." -f $_.Exception.Message.Trim(), $done)
    }

    # Verification reads everything back from disk.
    foreach ($result in $results) {
        $actual = Get-Sha256 $result.Path
        if ($actual -ne $result.Expected) {
            Fail "SHA256 mismatch after copy: $(Get-RelativePath $result.Path) is $actual, expected $($result.Expected)"
        }
    }
    foreach ($skill in $skillResults) {
        $mentioned = Get-MentionedVersions (Read-Utf8File $skill.Path).Text
        $stale = @($mentioned | Where-Object { $_ -cne $version })
        if ($mentioned.Count -eq 0 -or $stale.Count -gt 0) {
            Fail "$(Get-RelativePath $skill.Path) still mentions $($stale -join ', ') instead of $version"
        }
    }

    $exeSize = (Get-Item -LiteralPath $exe).Length
    Write-Info '==================== release manifest ===================='
    Write-Info "version        : $version"
    Write-Info "exe            : $(Get-RelativePath $exe) ($build)"
    Write-Info "exe sha256     : $exeHash ($exeSize bytes)"
    Write-Info "$HelpName sha256 : $helpHash"
    Write-Info "checks         : fmt passed; tests $tests"
    Write-Info "msvc           : $(Format-Msvc $msvc)"
    Write-Info 'destinations   :'
    foreach ($result in $results) {
        Write-Info ('  {0}  {1}  [{2}]' -f $result.Expected, (Get-RelativePath $result.Path), $result.State)
    }
    foreach ($skill in $skillResults) {
        Write-Info ('  {0}  {1}  [{2}: any_txt_parser {3} -> {4}]' -f (Get-Sha256 $skill.Path), (Get-RelativePath $skill.Path), $skill.State, $skill.Before, $version)
    }
    foreach ($skipped in $plan.Skipped) {
        Write-Info ('  {0}  {1}  [skipped: folder missing]' -f ('-' * 64), $skipped)
    }
    if ($NoSkills) { Write-Info '  skill folders not touched (-NoSkills)' }
    Write-Info ('result         : OK, {0} copies verified, {1} SKILL.md checked, {2} warning(s)' -f $results.Count, $skillResults.Count, $script:WarningCount)
}

$exitCode = 0
try {
    Invoke-Release
} catch {
    [Console]::Error.WriteLine('[release] ERROR: ' + $_.Exception.Message)
    $exitCode = 1
}
exit $exitCode
