[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][ValidateSet('Install','Uninstall')][string]$Mode,
    [Parameter(Mandatory=$true)][string]$InstallRoot,
    [ValidateSet(0,1)][int]$EnableStartup = 1
)
$ErrorActionPreference = 'Stop'
try {
    $root = (Resolve-Path -LiteralPath $InstallRoot).Path
    $gui = Join-Path $root 'bin/rusty-win32.exe'
    $hostExe = Join-Path $root 'bin/rusty-native-host.exe'
    $startup = '"' + $gui + '" --background'
    $runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
    $valueName = 'RustyDownloadManager'
    $current = if(Test-Path -LiteralPath $runKey) { (Get-Item -LiteralPath $runKey).GetValue($valueName) } else { $null }
    $ownerPath = Join-Path $env:LOCALAPPDATA 'RustyIDM/browser/registration-owner.json'
    if ($Mode -eq 'Install') {
        if ($current -and $current -ne $startup) { throw 'A different startup entry owns the Rusty name. It was preserved.' }
        if (Test-Path -LiteralPath $ownerPath) {
            $owner = Get-Content -Raw -LiteralPath $ownerPath | ConvertFrom-Json
            if($owner.app -ne 'RustyIDM' -or $owner.schema -ne 1 -or $owner.extension_id -cnotmatch '^[a-p]{32}$') { throw 'The existing Chrome ownership record is invalid. It was preserved.' }
            & (Join-Path $PSScriptRoot 'register-chrome-host.ps1') -ExtensionId $owner.extension_id -HostExe $hostExe -GuiExe $gui
        }
        if ($EnableStartup -eq 1) {
            New-Item -Path $runKey -Force | Out-Null
            New-ItemProperty -LiteralPath $runKey -Name $valueName -Value $startup -PropertyType String -Force | Out-Null
        } elseif ($current -eq $startup) { Remove-ItemProperty -LiteralPath $runKey -Name $valueName }
    } else {
        if($current -eq $startup) { Remove-ItemProperty -LiteralPath $runKey -Name $valueName }
        if(Test-Path -LiteralPath $ownerPath) {
            $owner = Get-Content -Raw -LiteralPath $ownerPath | ConvertFrom-Json
            if($owner.host_exe -eq $hostExe -and $owner.gui_exe -eq $gui) {
                & (Join-Path $PSScriptRoot 'unregister-chrome-host.ps1')
            }
        }
    }
    exit 0
} catch {
    Write-Error -ErrorAction Continue $_.Exception.Message
    exit 1
}
