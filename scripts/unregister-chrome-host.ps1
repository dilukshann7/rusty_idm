[CmdletBinding(SupportsShouldProcess=$true)]
param()
$ErrorActionPreference = 'Stop'
if (-not $env:LOCALAPPDATA) { throw 'LOCALAPPDATA is unavailable.' }
$installRoot = Join-Path $env:LOCALAPPDATA 'RustyIDM/browser'
$manifestPath = Join-Path $installRoot 'native-host.json'
$configPath = Join-Path $installRoot 'host-config.json'
$ownerPath = Join-Path $installRoot 'registration-owner.json'
$registryPath = 'HKCU:\Software\Google\Chrome\NativeMessagingHosts\dev.rusty_idm.chrome'
if (-not (Test-Path -LiteralPath $ownerPath -PathType Leaf)) { throw 'No Rusty ownership record. Nothing removed.' }
$owner = Get-Content -Raw -LiteralPath $ownerPath | ConvertFrom-Json
$manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
$config = Get-Content -Raw -LiteralPath $configPath | ConvertFrom-Json
if ($owner.app -ne 'RustyIDM' -or $owner.schema -ne 1 -or $owner.manifest -ne $manifestPath -or $manifest.name -ne 'dev.rusty_idm.chrome' -or $manifest.path -ne $owner.host_exe -or $config.host_exe -ne $owner.host_exe -or $config.gui_exe -ne $owner.gui_exe -or $config.extension_id -ne $owner.extension_id -or @($manifest.allowed_origins).Count -ne 1 -or $manifest.allowed_origins[0] -ne "chrome-extension://$($owner.extension_id)/") { throw 'Browser files do not match their Rusty owner. Nothing removed.' }
if (Test-Path -LiteralPath $registryPath) {
    if ((Get-Item -LiteralPath $registryPath).GetValue('') -ne $manifestPath) { throw 'A different registration owns this registry key. Nothing removed.' }
    if (@((Get-Item -LiteralPath $registryPath).GetSubKeyNames()).Count -ne 0 -or @((Get-Item -LiteralPath $registryPath).GetValueNames() | Where-Object { $_ -ne '' }).Count -ne 0) { throw 'Registration contains unrelated entries. Nothing removed.' }
}
if ($PSCmdlet.ShouldProcess($registryPath, 'Remove only this owned Rusty registration and its three configuration files')) {
    if (Test-Path -LiteralPath $registryPath) { Remove-Item -LiteralPath $registryPath }
    foreach ($ownedFile in @($manifestPath, $configPath, $ownerPath)) { Remove-Item -LiteralPath $ownedFile }
    Write-Output 'Rusty Chrome native host unregistered for the current user.'
}
