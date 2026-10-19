[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
[Windows.Forms.Application]::EnableVisualStyles()
$root = Split-Path -Parent $PSScriptRoot
$extensionPath = Join-Path $root 'chrome-extension'
$form = New-Object Windows.Forms.Form
$form.Text = 'Rusty Download Manager - Chrome setup'
$form.ClientSize = New-Object Drawing.Size(560,350)
$form.StartPosition = 'CenterScreen'
$form.FormBorderStyle = 'FixedDialog'; $form.MaximizeBox = $false
$iconPath = Join-Path $root 'branding/rusty.ico'
if(Test-Path -LiteralPath $iconPath) { $form.Icon = New-Object Drawing.Icon($iconPath) }
$description = New-Object Windows.Forms.Label
$description.Text = "Connect Chrome once. Rusty will then catch supported public downloads while running in the background.`r`n`r`n1. Open Chrome extensions and enable Developer mode.`r`n2. Choose Load unpacked and select the folder below.`r`n3. Paste the Rusty extension ID, then click Connect.`r`n4. Open Rusty's Chrome popup, enable Capture public downloads, and allow website access."
$description.SetBounds(16,14,528,132); $form.Controls.Add($description)
$folder = New-Object Windows.Forms.TextBox
$folder.Text = $extensionPath; $folder.ReadOnly = $true
$folder.SetBounds(16,150,528,24); $form.Controls.Add($folder)
$open = New-Object Windows.Forms.Button
$open.Text = 'Open Chrome extensions'; $open.SetBounds(16,184,180,28)
$open.Add_Click({ try { Start-Process -FilePath 'chrome.exe' -ArgumentList 'chrome://extensions/' } catch { [Windows.Forms.MessageBox]::Show('Open chrome://extensions in Chrome.','Chrome setup') | Out-Null } })
$form.Controls.Add($open)
$copy = New-Object Windows.Forms.Button
$copy.Text = 'Copy folder path'; $copy.SetBounds(206,184,150,28)
$copy.Add_Click({ [Windows.Forms.Clipboard]::SetText($extensionPath) }); $form.Controls.Add($copy)
$label = New-Object Windows.Forms.Label
$label.Text = 'Chrome extension ID'; $label.SetBounds(16,228,145,20); $form.Controls.Add($label)
$idBox = New-Object Windows.Forms.TextBox
$idBox.SetBounds(164,224,380,24); $form.Controls.Add($idBox)
$ownerPath = Join-Path $env:LOCALAPPDATA 'RustyIDM/browser/registration-owner.json'
try {
    if(Test-Path -LiteralPath $ownerPath) {
        $owner = Get-Content -Raw -LiteralPath $ownerPath | ConvertFrom-Json
        if($owner.app -eq 'RustyIDM' -and $owner.extension_id -cmatch '^[a-p]{32}$') { $idBox.Text = $owner.extension_id }
    }
} catch { }
$connect = New-Object Windows.Forms.Button
$connect.Text = 'Connect'; $connect.SetBounds(354,270,90,28)
$connect.Add_Click({
    try {
        $id = $idBox.Text.Trim()
        if($id -cnotmatch '^[a-p]{32}$') { throw 'Paste the 32-letter extension ID shown on chrome://extensions.' }
        & (Join-Path $PSScriptRoot 'register-chrome-host.ps1') -ExtensionId $id -HostExe (Join-Path $root 'bin/rusty-native-host.exe') -GuiExe (Join-Path $root 'bin/rusty-win32.exe')
        [Windows.Forms.MessageBox]::Show("Connected. Open Rusty's Chrome popup and enable Capture public downloads. Approve website access when Chrome asks.`r`n`r`nUse the tray icon to open or exit Rusty.",'Rusty Download Manager') | Out-Null
    } catch { [Windows.Forms.MessageBox]::Show($_.Exception.Message,'Could not connect') | Out-Null }
}); $form.Controls.Add($connect)
$close = New-Object Windows.Forms.Button
$close.Text = 'Close'; $close.SetBounds(454,270,90,28)
$close.Add_Click({ $form.Close() }); $form.Controls.Add($close)
$form.AcceptButton = $connect; $form.CancelButton = $close
try { [void]$form.ShowDialog() } finally { $form.Dispose() }
