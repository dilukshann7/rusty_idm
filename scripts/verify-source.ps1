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
    $expectedPackages = @('rusty-engine', 'rusty-win32', 'rusty-browser-bridge') | Sort-Object
    $actualPackages = @($metadata.packages | Where-Object { $_.id -in $metadata.workspace_members } | ForEach-Object { $_.name }) | Sort-Object
    if (Compare-Object $expectedPackages $actualPackages) { throw 'Unexpected workspace package set.' }

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

    foreach ($file in @('README.md', 'docs/implementation-progress.md', 'docs/native-gui-validation.md')) {
        $document = Join-Path $taskRoot $file
        $body = Get-Content -Raw -LiteralPath $document
        foreach ($match in [regex]::Matches($body, '\]\(([^)#]+)(?:#[^)]*)?\)')) {
            $link = $match.Groups[1].Value
            if ($link -match '^[a-zA-Z]+:') { continue }
            $resolved = Join-Path (Split-Path -Parent $document) $link
            if (-not (Test-Path -LiteralPath $resolved)) { throw "Broken document link in ${file}: $link" }
        }
    }
    Write-Output ('Source checks passed: {0} Rust files; workspace manifests/lock; XML/resource IDs/icon assets; handoff links.' -f $rustSources.Count)
    Write-Output 'This script performs no compilation, Rust tests, application, fixture or dev-server launch.'
}
finally { Pop-Location }
