param()
$ErrorActionPreference = 'Stop'
$taskRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
Push-Location -LiteralPath $taskRoot
try {
    $cargoTool = Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe'
    $formatTool = Join-Path $env:USERPROFILE '.cargo/bin/rustfmt.exe'
    if (-not (Test-Path -LiteralPath $cargoTool)) { $cargoTool = (Get-Command cargo -ErrorAction Stop).Source }
    if (-not (Test-Path -LiteralPath $formatTool)) { $formatTool = (Get-Command rustfmt -ErrorAction Stop).Source }

    $metadataText = & $cargoTool metadata --locked --offline --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Cargo manifest inspection failed.' }
    $metadata = $metadataText | ConvertFrom-Json
    $expectedPackages = @('rusty-engine', 'rusty-win32', 'rusty-browser-bridge', 'rusty-native-host') | Sort-Object
    $actualPackages = @($metadata.packages | Where-Object { $_.id -in $metadata.workspace_members } | ForEach-Object { $_.name }) | Sort-Object
    if (Compare-Object $expectedPackages $actualPackages) { throw 'Unexpected workspace package set.' }
    $versions = @($metadata.packages | Where-Object { $_.id -in $metadata.workspace_members } | ForEach-Object { $_.version } | Select-Object -Unique)
    if ($versions.Count -ne 1) { throw 'Workspace package versions differ.' }
    $releaseVersion = $versions[0]
    $browserVersion = ($releaseVersion -split '-', 2)[0]
    foreach ($browser in @('chrome', 'brave', 'edge', 'firefox')) {
        $manifestFile = if ($browser -eq 'chrome') { 'extensions/chrome/manifest.json' } else { "extensions/profiles/$browser/manifest.json" }
        $extensionManifest = Get-Content -Raw -LiteralPath (Join-Path $taskRoot $manifestFile) | ConvertFrom-Json
        if ($extensionManifest.version -ne $browserVersion) { throw "Extension version differs from Cargo: $manifestFile" }
        if ($browser -ne 'firefox' -and $releaseVersion.Contains('-') -and $extensionManifest.version_name -ne $releaseVersion) { throw "Extension beta label differs from Cargo: $manifestFile" }
    }
    $packageVersion = & python -c 'from tools.package_release import VERSION; print(VERSION)'
    if ($LASTEXITCODE -ne 0 -or $packageVersion -ne $releaseVersion) { throw 'Release packager version differs from Cargo.' }
    $installerSource = Get-Content -Raw -LiteralPath (Join-Path $taskRoot 'installer/windows.iss')
    if (-not $installerSource.Contains('#define AppVersion "' + $releaseVersion + '"')) {
        throw 'Release packager or installer version differs from Cargo.'
    }

    $rustSources = (Get-ChildItem -LiteralPath (Join-Path $taskRoot 'crates') -Recurse -Filter '*.rs').FullName
    & $formatTool --check --edition 2021 @rustSources
    if ($LASTEXITCODE -ne 0) { throw 'Rust formatting/syntax check failed.' }

    $rcPath = Join-Path $taskRoot 'crates/win32/resources/app.rc'
    $resourceText = Get-Content -Raw -LiteralPath $rcPath
    $manifests = [regex]::Matches($resourceText, '(?m)^\s*1\s+RT_MANIFEST\s+"([^"]+)"')
    if ($manifests.Count -ne 1) { throw 'Expected exactly one embedded application manifest.' }
    $manifestPath = Join-Path (Join-Path $taskRoot 'crates/win32') $manifests[0].Groups[1].Value
    [xml]$manifest = Get-Content -Raw -LiteralPath $manifestPath
    if ($manifest.assembly.manifestVersion -ne '1.0') { throw 'Invalid application manifest root.' }

    foreach ($match in [regex]::Matches($resourceText, '(?m)^\s*\d+\s+ICON\s+"([^"]+)"')) {
        $iconPath = Join-Path (Join-Path $taskRoot 'crates/win32') $match.Groups[1].Value
        $iconBytes = [System.IO.File]::ReadAllBytes($iconPath)
        if ($iconBytes.Length -lt 6 -or [BitConverter]::ToUInt16($iconBytes, 0) -ne 0 -or [BitConverter]::ToUInt16($iconBytes, 2) -ne 1) { throw "Invalid ICO file: $iconPath" }
        $frameCount = [BitConverter]::ToUInt16($iconBytes, 4)
        if ($frameCount -eq 0 -or $iconBytes.Length -lt (6 + 16 * $frameCount)) { throw "Incomplete ICO directory: $iconPath" }
        for ($frame = 0; $frame -lt $frameCount; $frame++) {
            $entry = 6 + 16 * $frame
            $frameSize = [BitConverter]::ToUInt32($iconBytes, $entry + 8)
            $frameOffset = [BitConverter]::ToUInt32($iconBytes, $entry + 12)
            if ($frameOffset -lt (6 + 16 * $frameCount) -or $frameSize -eq 0 -or ([long]$frameOffset + $frameSize) -gt $iconBytes.Length) { throw "Incomplete ICO frame: $iconPath" }
        }
    }

    $commands = Get-Content -Raw -LiteralPath (Join-Path $taskRoot 'crates/win32/src/commands.rs')
    $ids = @{}
    foreach ($match in [regex]::Matches($commands, 'pub const \w+: (?:u16|i32) = (\d+);')) { $ids[[int]$match.Groups[1].Value] = $true }
    foreach ($match in [regex]::Matches($resourceText, '(?m)^\s*(?:MENUITEM\s+"[^"]*",\s*|(?:EDITTEXT|\w*PUSHBUTTON|CONTROL|LTEXT)\s+(?:"[^"]*",\s*)?)(\d+)')) {
        $id = [int]$match.Groups[1].Value
        if ($id -gt 1000 -and -not $ids.ContainsKey($id)) { throw "Resource control/command $id has no Rust ID." }
    }

    $documents = @('README.md', 'THIRD_PARTY.md') + @(Get-ChildItem -LiteralPath (Join-Path $taskRoot 'docs') -Recurse -Filter '*.md' | ForEach-Object { $_.FullName.Substring($taskRoot.Length + 1) })
    foreach ($file in $documents) {
        $document = Join-Path $taskRoot $file
        $body = Get-Content -Raw -LiteralPath $document
        foreach ($match in [regex]::Matches($body, '\]\(([^)#]+)(?:#[^)]*)?\)')) {
            $link = $match.Groups[1].Value
            if ($link -match '^[a-zA-Z]+:') { continue }
            $resolved = Join-Path (Split-Path -Parent $document) $link
            if (-not (Test-Path -LiteralPath $resolved)) { throw "Broken document link in ${file}: $link" }
        }
    }
    & (Join-Path $PSScriptRoot 'verify-chrome-source.ps1')
    Write-Output ('Source checks passed: {0} Rust files; workspace manifests/lock; XML/resource IDs/icon assets; handoff links.' -f $rustSources.Count)
    Write-Output 'This script performs no compilation, Rust tests, application, fixture or dev-server launch.'
}
finally { Pop-Location }
