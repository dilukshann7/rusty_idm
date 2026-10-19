[CmdletBinding(SupportsShouldProcess=$true)]
param(
    [Parameter(Mandatory=$true)][ValidatePattern('^[a-p]{32}$')][string]$ExtensionId,
    [Parameter(Mandatory=$true)][string]$HostExe,
    [Parameter(Mandatory=$true)][string]$GuiExe
)
$ErrorActionPreference = 'Stop'
if ($ExtensionId -cnotmatch '^[a-p]{32}$') { throw 'Supply one exact lowercase Chrome extension ID.' }
foreach ($binary in @($HostExe, $GuiExe)) {
    if (-not [IO.Path]::IsPathRooted($binary) -or $binary -notmatch '^(?:[a-zA-Z]:[\\/]|\\\\)' -or [IO.Path]::GetExtension($binary) -ne '.exe' -or -not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw 'Supply existing absolute .exe paths for the native host and GUI.' }
}
if (-not $env:LOCALAPPDATA) { throw 'LOCALAPPDATA is unavailable.' }
$hostPath = (Resolve-Path -LiteralPath $HostExe).Path
$guiPath = (Resolve-Path -LiteralPath $GuiExe).Path
$installRoot = Join-Path $env:LOCALAPPDATA 'RustyIDM/browser'
$manifestPath = Join-Path $installRoot 'native-host.json'
$configPath = Join-Path $installRoot 'host-config.json'
$ownerPath = Join-Path $installRoot 'registration-owner.json'
$registryPath = 'HKCU:\Software\Google\Chrome\NativeMessagingHosts\dev.rusty_idm.chrome'
if (Test-Path -LiteralPath $registryPath) {
    $registered = (Get-Item -LiteralPath $registryPath).GetValue('')
    if ($registered -ne $manifestPath -or -not (Test-Path -LiteralPath $ownerPath)) { throw 'A different native host registration owns this name. Nothing changed.' }
}
if ((Test-Path -LiteralPath $configPath) -or (Test-Path -LiteralPath $manifestPath)) {
    if (-not (Test-Path -LiteralPath $ownerPath)) { throw 'Existing browser files have no Rusty ownership record. Nothing changed.' }
    $owner = Get-Content -Raw -Encoding UTF8 -LiteralPath $ownerPath | ConvertFrom-Json
    if ($owner.app -ne 'RustyIDM' -or $owner.schema -ne 1 -or $owner.manifest -ne $manifestPath -or $owner.extension_id -ne $ExtensionId) { throw 'Existing browser files belong to a different registration. Nothing changed.' }
}
$manifest = [ordered]@{ name='dev.rusty_idm.chrome'; description='Rusty Download Manager browser bridge'; path=$hostPath; type='stdio'; allowed_origins=@("chrome-extension://$ExtensionId/") } | ConvertTo-Json -Depth 4
$config = [ordered]@{ extension_id=$ExtensionId; host_exe=$hostPath; gui_exe=$guiPath } | ConvertTo-Json
$ownership = [ordered]@{ app='RustyIDM'; schema=1; manifest=$manifestPath; extension_id=$ExtensionId; host_exe=$hostPath; gui_exe=$guiPath } | ConvertTo-Json
if ($PSCmdlet.ShouldProcess($registryPath, 'Register the Rusty native host for this user and exact extension ID')) {
    New-Item -ItemType Directory -Path $installRoot -Force | Out-Null
    $utf8 = New-Object System.Text.UTF8Encoding($false)
    [IO.File]::WriteAllText($manifestPath, $manifest, $utf8)
    [IO.File]::WriteAllText($configPath, $config, $utf8)
    [IO.File]::WriteAllText($ownerPath, $ownership, $utf8)
    New-Item -Path $registryPath -Force | Out-Null
    Set-Item -LiteralPath $registryPath -Value $manifestPath
    Write-Output 'Rusty Chrome native host registered for the current user.'
}
