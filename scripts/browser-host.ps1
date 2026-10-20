[CmdletBinding(SupportsShouldProcess=$true)]
param(
    [ValidateSet('Register','Unregister')][string]$Mode='Register',
    [Parameter(Mandatory=$true)][ValidateSet('chrome','brave','edge','firefox')][string]$Browser,
    [string]$ExtensionId,
    [string]$HostExe,
    [string]$GuiExe
)
$ErrorActionPreference='Stop'
$vendors=@{chrome='Google\Chrome';brave='BraveSoftware\Brave-Browser';edge='Microsoft\Edge';firefox='Mozilla'}
$suffix=if($Browser -eq 'chrome'){''}else{'/'+$Browser}
$root=Join-Path $env:LOCALAPPDATA ('RustyIDM/browser'+$suffix)
$manifestPath=Join-Path $root 'native-host.json'
$configPath=Join-Path $root 'host-config.json'
$ownerPath=Join-Path $root 'registration-owner.json'
$registryPath='HKCU:\Software\'+$vendors[$Browser]+'\NativeMessagingHosts\dev.rusty_idm.'+$Browser
$hostName='dev.rusty_idm.'+$Browser
$files=@($manifestPath,$configPath,$ownerPath)
$before=@{};foreach($file in $files){$before[$file]=if(Test-Path -LiteralPath $file){[IO.File]::ReadAllBytes($file)}else{$null}}
$registered=if(Test-Path -LiteralPath $registryPath){(Get-Item -LiteralPath $registryPath).GetValue('')}else{$null}
$owner=$null;$config=$null;$manifest=$null
if(@($files | Where-Object {Test-Path -LiteralPath $_}).Count -gt 0){
    if(@($files | Where-Object {Test-Path -LiteralPath $_}).Count -ne 3){throw 'Incomplete browser ownership data was preserved. Repair it before registering.'}
    $owner=Get-Content -Raw -Encoding UTF8 -LiteralPath $ownerPath | ConvertFrom-Json
    $config=Get-Content -Raw -Encoding UTF8 -LiteralPath $configPath | ConvertFrom-Json
    $manifest=Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json
    if($owner.app -ne 'RustyIDM' -or $owner.schema -ne 1 -or $owner.manifest -ne $manifestPath -or $manifest.name -ne $hostName -or $manifest.path -ne $owner.host_exe -or $config.host_exe -ne $owner.host_exe -or $config.gui_exe -ne $owner.gui_exe -or $config.extension_id -ne $owner.extension_id){throw 'Existing browser ownership data does not match. It was preserved.'}
    $ids=@($config.extension_id)+@($config.extension_ids) | Where-Object {$_} | Select-Object -Unique
    $allowed=if($Browser -eq 'firefox'){@($manifest.allowed_extensions)}else{@($manifest.allowed_origins | ForEach-Object {if($_ -cmatch '^chrome-extension://([a-p]{32})/$'){$Matches[1]}else{'invalid'}})}
    if(Compare-Object @($ids | Sort-Object) @($allowed | Sort-Object)){throw 'Browser caller allowlists differ. Nothing changed.'}
}
if($registered -and ($registered -ne $manifestPath -or -not $owner)){throw 'Another registration owns this browser host. Nothing changed.'}
if($Mode -eq 'Unregister' -and -not $owner){return}
if($Mode -eq 'Register'){
    if(($Browser -eq 'firefox' -and $ExtensionId -cnotmatch '^[a-zA-Z0-9@._{}-]{3,200}$') -or ($Browser -ne 'firefox' -and $ExtensionId -cnotmatch '^[a-p]{32}$')){throw 'Enter this browser extension exact ID.'}
    foreach($binary in @($HostExe,$GuiExe)){if(-not [IO.Path]::IsPathRooted($binary) -or [IO.Path]::GetExtension($binary) -ine '.exe' -or -not (Test-Path -LiteralPath $binary -PathType Leaf)){throw 'Existing absolute host and application .exe paths are required.'}}
    $sourceHost=(Resolve-Path -LiteralPath $HostExe).Path;$gui=(Resolve-Path -LiteralPath $GuiExe).Path
    $hostPath=if($Browser -eq 'chrome'){$sourceHost}else{Join-Path $root 'rusty-native-host.exe'}
    if($Browser -ne 'chrome'){
        if((Test-Path -LiteralPath $hostPath) -and (-not $owner -or $owner.host_exe -ne $hostPath)){throw 'An unowned native host executable was preserved.'}
        $before[$hostPath]=if(Test-Path -LiteralPath $hostPath){[IO.File]::ReadAllBytes($hostPath)}else{$null}
    }
    $primary=if($owner){$owner.extension_id}else{$ExtensionId}
    $ids=@($primary)+@($config.extension_ids)+@($ExtensionId) | Where-Object {$_} | Select-Object -Unique
    if(@($ids).Count -gt 32){throw 'Maximum 32 registered extension profiles per browser.'}
    $nextConfig=@{browser=$Browser;extension_id=$primary;extension_ids=@($ids | Where-Object {$_ -ne $primary});host_exe=$hostPath;gui_exe=$gui}
    $nextManifest=@{name=$hostName;description='Rusty Download Manager browser bridge';path=$hostPath;type='stdio'}
    if($Browser -eq 'firefox'){$nextManifest.allowed_extensions=@($ids)}else{$nextManifest.allowed_origins=@($ids | ForEach-Object {'chrome-extension://'+$_+'/'})}
    $nextOwner=@{app='RustyIDM';schema=1;manifest=$manifestPath;extension_id=$primary;extension_ids=@($ids | Where-Object {$_ -ne $primary});host_exe=$hostPath;gui_exe=$gui;browser=$Browser}
}
if(-not $PSCmdlet.ShouldProcess($registryPath,($Mode+' owned Rusty browser host'))){return}
$utf8=New-Object System.Text.UTF8Encoding($false)
function Write-Atomic([string]$Path,[byte[]]$Bytes){
    $temporary=$Path+'.'+[Guid]::NewGuid().ToString('N')+'.tmp'
    try{[IO.File]::WriteAllBytes($temporary,$Bytes);if(Test-Path -LiteralPath $Path){[IO.File]::Replace($temporary,$Path,$null)}else{[IO.File]::Move($temporary,$Path)}}finally{if(Test-Path -LiteralPath $temporary){Remove-Item -LiteralPath $temporary}}
}
try{
    if($Mode -eq 'Register'){
        New-Item -ItemType Directory -Path $root -Force | Out-Null
        if($Browser -ne 'chrome' -and (-not (Test-Path -LiteralPath $hostPath) -or (Get-FileHash -LiteralPath $sourceHost).Hash -ne (Get-FileHash -LiteralPath $hostPath).Hash)){Copy-Item -LiteralPath $sourceHost -Destination $hostPath -Force}
        foreach($entry in @(@($manifestPath,$nextManifest),@($configPath,$nextConfig),@($ownerPath,$nextOwner))){$bytes=$utf8.GetBytes(($entry[1] | ConvertTo-Json -Depth 8));Write-Atomic $entry[0] $bytes}
        New-Item -Path $registryPath -Force | Out-Null;Set-Item -LiteralPath $registryPath -Value $manifestPath
    }else{
        if(Test-Path -LiteralPath $registryPath){$key=Get-Item -LiteralPath $registryPath;if(@($key.GetSubKeyNames()).Count -or @($key.GetValueNames() | Where-Object {$_ -ne ''}).Count){throw 'Unrelated registration entries were preserved.'};Remove-Item -LiteralPath $registryPath}
        foreach($file in $files){Remove-Item -LiteralPath $file}
    }
}catch{
    $failure=$_.Exception.Message;$rollback=@()
    foreach($file in $before.Keys){try{if($null -ne $before[$file]){Write-Atomic $file $before[$file]}elseif(Test-Path -LiteralPath $file){Remove-Item -LiteralPath $file}}catch{$rollback+=$_.Exception.Message}}
    try{if($null -ne $registered){New-Item -Path $registryPath -Force | Out-Null;Set-Item -LiteralPath $registryPath -Value $registered}elseif(Test-Path -LiteralPath $registryPath){Remove-Item -LiteralPath $registryPath}}catch{$rollback+=$_.Exception.Message}
    if($rollback.Count){throw ($failure+' Rollback needs repair: '+($rollback -join '; '))};throw $failure
}
Write-Output ('Rusty '+$Browser+' host '+$Mode.ToLowerInvariant()+' completed.')
