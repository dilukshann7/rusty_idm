# Optional Pester suite; execution is deferred until explicitly authorized.
Describe 'Rusty Chrome registration' {
    BeforeAll { $register = Join-Path $PSScriptRoot '../register-chrome-host.ps1'; $unregister = Join-Path $PSScriptRoot '../unregister-chrome-host.ps1' }
    BeforeEach { $fixtureRoot = Join-Path $TestDrive ([Guid]::NewGuid().ToString()); [IO.Directory]::CreateDirectory($fixtureRoot) | Out-Null }
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
            $env:LOCALAPPDATA = $fixtureRoot
            $hostFile = Join-Path $fixtureRoot 'தமிழ் host.exe'; $guiFile = Join-Path $fixtureRoot 'தமிழ் gui.exe'
            [IO.File]::WriteAllBytes($hostFile, [byte[]]@(0)); [IO.File]::WriteAllBytes($guiFile, [byte[]]@(0))
            Mock Test-Path { $false } -ParameterFilter { $LiteralPath -like 'HKCU:*' }
            Mock New-Item { } -ParameterFilter { $Path -like 'HKCU:*' }
            Mock Set-Item { } -ParameterFilter { $LiteralPath -like 'HKCU:*' }
            & $register -ExtensionId ('a' * 32) -HostExe $hostFile -GuiExe $guiFile -Confirm:$false
            $config = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $fixtureRoot 'RustyIDM/browser/host-config.json') | ConvertFrom-Json
            $config.gui_exe | Should -Be $guiFile
            $config.host_exe | Should -Be $hostFile
        } finally { $env:LOCALAPPDATA = $oldLocal }
    }
    It 'refuses uninstall when the registry belongs to a different registration' {
        $oldLocal = $env:LOCALAPPDATA
        try {
            $env:LOCALAPPDATA = $fixtureRoot
            $ownedRoot = Join-Path $fixtureRoot 'RustyIDM/browser'; [IO.Directory]::CreateDirectory($ownedRoot) | Out-Null
            $manifestPath = Join-Path $ownedRoot 'native-host.json'; $extensionId = 'a' * 32
            $owner = @{ app='RustyIDM'; schema=1; manifest=$manifestPath; extension_id=$extensionId; host_exe='C:\Rusty\host.exe'; gui_exe='C:\Rusty\gui.exe' }
            $manifest = @{ name='dev.rusty_idm.chrome'; path=$owner.host_exe; allowed_origins=@("chrome-extension://$extensionId/") }
            $config = @{ extension_id=$extensionId; host_exe=$owner.host_exe; gui_exe=$owner.gui_exe }
            [IO.File]::WriteAllText((Join-Path $ownedRoot 'registration-owner.json'), ($owner | ConvertTo-Json))
            [IO.File]::WriteAllText($manifestPath, ($manifest | ConvertTo-Json))
            [IO.File]::WriteAllText((Join-Path $ownedRoot 'host-config.json'), ($config | ConvertTo-Json))
            Mock Test-Path { $true } -ParameterFilter { $LiteralPath -like 'HKCU:*' }
            Mock Get-Item { $entry = [pscustomobject]@{}; $entry | Add-Member -MemberType ScriptMethod -Name GetValue -Value { param($name) 'C:\Other\host.json' }; $entry } -ParameterFilter { $LiteralPath -like 'HKCU:*' }
            Mock Remove-Item { throw 'must not remove' }
            { & $unregister -Confirm:$false } | Should -Throw
            Should -Invoke Remove-Item -Times 0
        } finally { $env:LOCALAPPDATA = $oldLocal }
    }
    It 'WhatIf leaves registration and configuration unwritten' {
        $oldLocal = $env:LOCALAPPDATA
        try {
            $env:LOCALAPPDATA = $fixtureRoot
            $hostFile = Join-Path $fixtureRoot 'host.exe'; $guiFile = Join-Path $fixtureRoot 'gui.exe'
            [IO.File]::WriteAllBytes($hostFile, [byte[]]@(0)); [IO.File]::WriteAllBytes($guiFile, [byte[]]@(0))
            Mock Test-Path { $false } -ParameterFilter { $LiteralPath -like 'HKCU:*' }
            Mock New-Item { throw 'must not write' }
            Mock Set-Item { throw 'must not write' }
            & $register -ExtensionId ('a' * 32) -HostExe $hostFile -GuiExe $guiFile -WhatIf
            Test-Path -LiteralPath (Join-Path $fixtureRoot 'RustyIDM/browser/host-config.json') | Should -BeFalse
            Should -Invoke New-Item -Times 0; Should -Invoke Set-Item -Times 0
        } finally { $env:LOCALAPPDATA = $oldLocal }
    }
}
