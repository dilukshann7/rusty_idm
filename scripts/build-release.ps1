[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$Compiler,
    [string]$TargetDir = (Join-Path $PSScriptRoot '../target')
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$compilerPath = (Resolve-Path -LiteralPath $Compiler).Path
if(Test-Path -LiteralPath (Join-Path $root 'dist/payload')) { throw 'Existing dist/payload must be preserved before making a new release.' }
Push-Location -LiteralPath $root
$oldFlags = $env:RUSTFLAGS
try {
    & (Join-Path $PSScriptRoot 'verify-source.ps1')
    $env:RUSTFLAGS = '-C target-feature=+crt-static'
    & "$env:USERPROFILE/.cargo/bin/cargo.exe" build --release --locked --offline -p rusty-win32 -p rusty-native-host --target-dir $TargetDir
    if($LASTEXITCODE -ne 0) { throw 'Optimized Windows build failed.' }
    $binaries = Join-Path $TargetDir 'release'
    foreach($name in @('rusty-win32.exe','rusty-native-host.exe')) {
        $info = (Get-Item -LiteralPath (Join-Path $binaries $name)).VersionInfo
        if($info.ProductName -ne 'Rusty Download Manager' -or $info.ProductVersion -ne '0.2.0') { throw "Incorrect product/version in $name" }
    }
    & python tools/package_release.py stage --binary-dir $binaries
    if($LASTEXITCODE -ne 0) { throw 'Packaging failed.' }
    & $compilerPath '/Qp' ("/DSourceRoot=" + $root) ("/DPayloadRoot=" + (Join-Path $root 'dist/payload')) (Join-Path $root 'installer/windows.iss')
    if($LASTEXITCODE -ne 0) { throw 'Installer compilation failed.' }
    $setupInfo = (Get-Item -LiteralPath (Join-Path $root 'dist/RustyDownloadManager-0.2.0-windows-x64-setup.exe')).VersionInfo
    if($setupInfo.ProductName.Trim() -ne 'Rusty Download Manager' -or $setupInfo.ProductVersion.Trim() -ne '0.2.0') { throw 'Incorrect setup product/version.' }
    & python tools/package_release.py finish
    if($LASTEXITCODE -ne 0) { throw 'Release manifest creation failed.' }
} finally { $env:RUSTFLAGS = $oldFlags; Pop-Location }
