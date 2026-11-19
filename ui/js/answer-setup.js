"use strict";

function answerSetupScript(apps, offline) {
  const log = String.raw`
function Write-SetupLog($Message) {
    Add-Content -LiteralPath "$env:windir\Panther\OrbitalSetup.log" -Value $Message -ErrorAction SilentlyContinue
}
`;
  const deprovision = apps.length ? `
try {
    $names = '${apps.join("','")}'
    foreach ($package in Get-AppxProvisionedPackage -Online -ErrorAction Stop) {
        if ($package.DisplayName -notin $names) { continue }
        try {
            Remove-AppxProvisionedPackage -Online -PackageName $package.PackageName -ErrorAction Stop | Out-Null
            Write-SetupLog "Deprovisioned $($package.PackageName)"
        } catch {
            Write-SetupLog "Could not deprovision $($package.PackageName): $_"
        }
    }
} catch {
    Write-SetupLog "Could not read provisioned apps: $_"
}
` : "";
  const network = offline ? String.raw`
try {
    $task = 'Orbital Restore Network'
    $adapters = @(Get-NetAdapter -ErrorAction Stop | Where-Object { $_.AdminStatus -eq 'Up' -and $_.InterfaceGuid })
    if ($adapters.Count -and -not (Get-ScheduledTask -TaskName $task -ErrorAction SilentlyContinue)) {
        $ids = ($adapters.InterfaceGuid | ForEach-Object { [guid]$_ }) -join ','
        $restore = ('$ids = "{0}" -split ","' -f $ids) + @'

try {
    $failed = $false
    foreach ($adapter in Get-NetAdapter -IncludeHidden -ErrorAction Stop) {
        if ($adapter.InterfaceGuid -and [string][guid]$adapter.InterfaceGuid -in $ids -and $adapter.AdminStatus -ne 'Up') {
            try {
                Enable-NetAdapter -InputObject $adapter -Confirm:$false -ErrorAction Stop
            } catch {
                $failed = $true
                Add-Content -LiteralPath "$env:windir\Panther\OrbitalSetup.log" -Value "Could not restore $($adapter.Name): $_" -ErrorAction SilentlyContinue
            }
        }
    }
    if ($failed) { exit 1 }
    Unregister-ScheduledTask -TaskName 'Orbital Restore Network' -Confirm:$false -ErrorAction Stop
} catch {
    Add-Content -LiteralPath "$env:windir\Panther\OrbitalSetup.log" -Value "Could not restore network adapters: $_" -ErrorAction SilentlyContinue
    exit 1
}
'@
        $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($restore))
        $action = New-ScheduledTaskAction -Execute (Join-Path $PSHOME 'powershell.exe') -Argument "-NoProfile -NonInteractive -EncodedCommand $encoded"
        $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)
        Register-ScheduledTask -TaskName $task -Action $action -Settings $settings -User 'SYSTEM' -RunLevel Highest -ErrorAction Stop | Out-Null
        foreach ($adapter in $adapters) {
            try {
                Disable-NetAdapter -InputObject $adapter -Confirm:$false -ErrorAction Stop
            } catch {
                Write-SetupLog "Could not disable $($adapter.Name): $_"
            }
        }
    }
} catch {
    Write-SetupLog "Could not set up offline first run: $_"
}
` : "";
  return log + deprovision + network;
}
