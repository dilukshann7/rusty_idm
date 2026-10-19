# Optional Pester suite; execution is deferred until explicitly authorized.
Describe 'Rusty Chrome registration' {
    BeforeAll { $register = Join-Path $PSScriptRoot '../register-chrome-host.ps1'; $unregister = Join-Path $PSScriptRoot '../unregister-chrome-host.ps1' }
    It 'rejects an invalid extension ID before making changes' {
        Mock New-Item { throw 'must not write' }
        { & $register -ExtensionId 'invalid' -HostExe 'C:\missing.exe' -GuiExe 'C:\missing.exe' } | Should -Throw
        Should -Invoke New-Item -Times 0
    }
    It 'rejects missing binaries without registration' {
        Mock New-Item { throw 'must not write' }
        { & $register -ExtensionId ('a' * 32) -HostExe 'C:\missing-rusty-host.exe' -GuiExe 'C:\missing-rusty-gui.exe' } | Should -Throw
        Should -Invoke New-Item -Times 0
    }
    It 'serializes spaced Unicode executable paths as JSON strings' {
        $oldLocal = $env:LOCALAPPDATA
        try {
            $env:LOCALAPPDATA = $TestDrive
            $hostFile = Join-Path $TestDrive 'தமிழ் host.exe'; $guiFile = Join-Path $TestDrive 'தமிழ் gui.exe'
            [IO.File]::WriteAllBytes($hostFile, [byte[]]@(0)); [IO.File]::WriteAllBytes($guiFile, [byte[]]@(0))
            Mock Test-Path { $false } -ParameterFilter { $LiteralPath -like 'HKCU:*' }
            Mock New-Item { } -ParameterFilter { $Path -like 'HKCU:*' }
            Mock Set-Item { } -ParameterFilter { $LiteralPath -like 'HKCU:*' }
            & $register -ExtensionId ('a' * 32) -HostExe $hostFile -GuiExe $guiFile -Confirm:$false
            $config = Get-Content -Raw -LiteralPath (Join-Path $TestDrive 'RustyIDM/browser/host-config.json') | ConvertFrom-Json
            $config.gui_exe | Should -Be $guiFile
            $config.host_exe | Should -Be $hostFile
        } finally { $env:LOCALAPPDATA = $oldLocal }
    }
    It 'refuses uninstall when the registry belongs to a different registration' {
        $oldLocal = $env:LOCALAPPDATA
        try {
            $env:LOCALAPPDATA = $TestDrive
            Mock Get-Item { [pscustomobject]@{} } -ParameterFilter { $LiteralPath -like 'HKCU:*' }
            # No valid ownership record in this isolated local app directory: must refuse.
            Mock Remove-Item { throw 'must not remove' }
            { & $unregister -Confirm:$false } | Should -Throw
            Should -Invoke Remove-Item -Times 0
        } finally { $env:LOCALAPPDATA = $oldLocal }
    }
}
