param()
$ErrorActionPreference = 'Stop'
$taskRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$extension = Join-Path $taskRoot 'extensions/chrome'
$manifest = Get-Content -Raw -LiteralPath (Join-Path $extension 'manifest.json') | ConvertFrom-Json
if ($manifest.manifest_version -ne 3 -or $manifest.minimum_chrome_version -ne '120' -or $manifest.background.type -ne 'module' -or $manifest.incognito -ne 'not_allowed') { throw 'Invalid extension platform contract.' }
$required = @('downloads','nativeMessaging','contextMenus','storage','alarms','webRequest','scripting','webNavigation')
if (Compare-Object ($required | Sort-Object) (@($manifest.permissions) | Sort-Object)) { throw 'Unexpected extension permissions.' }
if (Compare-Object (@('http://*/*','https://*/*') | Sort-Object) (@($manifest.optional_host_permissions) | Sort-Object)) { throw 'Unexpected optional host permissions.' }
if ($manifest.content_scripts -or $manifest.host_permissions -or $manifest.externally_connectable) { throw 'Unexpected extension access.' }
foreach ($relative in @($manifest.background.service_worker, $manifest.action.default_popup) + @($manifest.icons.psobject.Properties.Value)) {
    if ($relative -match '^(?:[a-z]+:|[\\/])' -or $relative -match '\.\.' -or -not (Test-Path -LiteralPath (Join-Path $extension $relative) -PathType Leaf)) { throw 'Missing or non-local extension asset.' }
}
foreach ($size in @(16,32,48,128)) {
    $bytes = [IO.File]::ReadAllBytes((Join-Path $extension "icons/rusty-$size.png"))
    if ($bytes.Length -lt 24 -or [BitConverter]::ToString($bytes[0..7]) -ne '89-50-4E-47-0D-0A-1A-0A') { throw 'Invalid icon PNG.' }
    $width = ($bytes[16] -shl 24) -bor ($bytes[17] -shl 16) -bor ($bytes[18] -shl 8) -bor $bytes[19]
    $height = ($bytes[20] -shl 24) -bor ($bytes[21] -shl 16) -bor ($bytes[22] -shl 8) -bor $bytes[23]
    if ($width -ne $size -or $height -ne $size) { throw 'Incorrect icon dimensions.' }
}
foreach ($page in Get-ChildItem -LiteralPath $extension -Filter '*.html') {
    $html = Get-Content -Raw -LiteralPath $page.FullName
    if ($html -match '(?i)<script[^>]*src=["''](?:https?:|//)' -or $html -match '(?i)\son\w+\s*=') { throw 'Remote or inline event script found.' }
    foreach ($match in [regex]::Matches($html, '(?:src|href)="([^"#]+)"')) {
        if ($match.Groups[1].Value -notmatch '^[a-z]+:' -and -not (Test-Path -LiteralPath (Join-Path $extension $match.Groups[1].Value))) { throw 'Broken popup asset link.' }
    }
}
foreach ($file in Get-ChildItem -LiteralPath $extension -Recurse -Filter '*.js') {
    & node --check $file.FullName
    if ($LASTEXITCODE -ne 0) { throw "JavaScript syntax error: $($file.Name)" }
    foreach ($match in [regex]::Matches((Get-Content -Raw -LiteralPath $file.FullName), 'from\s+[''"](\.[^''"]+)[''"]')) {
        if (-not (Test-Path -LiteralPath (Join-Path $file.DirectoryName $match.Groups[1].Value))) { throw 'Missing local JavaScript import.' }
    }
}
foreach ($relative in @('scripts/browser-host.ps1','scripts/register-chrome-host.ps1','scripts/unregister-chrome-host.ps1','scripts/verify-chrome-source.ps1','scripts/tests/chrome-host-registration.Tests.ps1','scripts/release-integration.ps1','scripts/chrome-integration.ps1','scripts/build-release.ps1','scripts/generate-branding.ps1')) {
    $tokens = $null; $errors = $null
    [System.Management.Automation.Language.Parser]::ParseFile((Join-Path $taskRoot $relative), [ref]$tokens, [ref]$errors) | Out-Null
    if ($errors.Count) { throw "PowerShell syntax errors in $relative" }
}
foreach($browser in @('brave','edge','firefox')) {
    $profile=Join-Path $taskRoot ('extensions/profiles/'+$browser)
    $m=Get-Content -Raw -LiteralPath (Join-Path $profile 'manifest.json') | ConvertFrom-Json
    if($m.manifest_version -ne 3 -or $m.incognito -ne 'not_allowed'){throw 'Invalid browser profile.'}
    if(Compare-Object ($required | Sort-Object) (@($m.permissions) | Sort-Object)){throw 'Unexpected browser profile permissions.'}
    if($browser -eq 'firefox' -and ($m.browser_specific_settings.gecko.id -ne 'rusty-download-manager@rusty.local' -or $m.background.scripts[0] -ne 'service-worker.js')){throw 'Invalid Firefox background or identity.'}
    & node --check (Join-Path $profile 'browser-config.js')
    if($LASTEXITCODE -ne 0){throw 'Invalid browser profile configuration.'}
}
Write-Output 'Chrome source checks passed: manifest, local assets, PNG sizes, JavaScript and PowerShell syntax. No tests, browser or setup executed.'
