$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$WarningPreference = 'SilentlyContinue'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)

function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    return [Security.Principal.WindowsPrincipal]::new($identity).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Assert-Administrator {
    if (-not (Test-Administrator)) { throw 'This removal requires administrator access. Close Orbital and run it as administrator.' }
}

function Get-DesktopRoots {
    @(
        'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
        'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall',
        'Registry::HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
        'Registry::HKEY_CURRENT_USER\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
    )
}

function Get-CatalogId($Entry) {
    if ($Entry.Publisher -notmatch '^Microsoft( Corporation)?$') { return '' }
    if ($Entry.DisplayName -eq 'Microsoft Edge') { return 'Microsoft.Edge' }
    if ($Entry.DisplayName -in @('Microsoft OneDrive', 'OneDrive')) { return 'Microsoft.OneDrive' }
    return ''
}

function New-App($Name, $Id, $Kind, $Version, $Details, $Blocked = '', $Catalog = '') {
    [pscustomobject]@{
        name = [string]$Name; id = [string]$Id; kind = $Kind; version = [string]$Version
        details = @($Details); selectable = -not [bool]$Blocked; blocked_reason = $Blocked; catalog_id = $Catalog
        requires_permanent = $Kind -eq 'capability' -or ($Kind -eq 'feature' -and $Version -eq 'Disabled')
    }
}

function New-Detail($Label, $Value) {
    [pscustomobject]@{ label = $Label; value = [string]$Value }
}

function Get-BundledPaths([string]$Catalog) {
    $paths = switch ($Catalog) {
        'Microsoft.Edge' {
            @{ Root = ${env:ProgramFiles(x86)}; Relative = 'Microsoft\Edge' }
            @{ Root = $env:ProgramFiles; Relative = 'Microsoft\Edge' }
            @{ Root = $env:LOCALAPPDATA; Relative = 'Microsoft\Edge\Application' }
        }
        'Microsoft.OneDrive' {
            @{ Root = $env:SystemRoot; Relative = 'System32\OneDriveSetup.exe' }
            @{ Root = $env:SystemRoot; Relative = 'SysWOW64\OneDriveSetup.exe' }
            @{ Root = $env:LOCALAPPDATA; Relative = 'Microsoft\OneDrive' }
            @{ Root = $env:ProgramFiles; Relative = 'Microsoft OneDrive' }
            @{ Root = ${env:ProgramFiles(x86)}; Relative = 'Microsoft OneDrive' }
        }
    }
    $paths | Where-Object { $_.Root }
}

function Get-NativeInventory {
    $apps = [Collections.Generic.List[object]]::new()
    $warnings = [Collections.Generic.List[string]]::new()
    $admin = Test-Administrator
    $parameters = @{ PackageTypeFilter = @('Main', 'Bundle', 'Framework') }
    if ($admin) { $parameters.AllUsers = $true }
    $packages = @(Get-AppxPackage @parameters)
    $provisioned = @()
    if ($admin) {
        try { $provisioned = @(Get-AppxProvisionedPackage -Online) }
        catch { $warnings.Add("Could not read provisioned apps: $($_.Exception.Message)") }
    } else {
        $warnings.Add('Showing apps for the current user and desktop programs. Run Orbital as administrator to include all users, provisioned apps and Windows components.')
    }
    $names = @(@($packages | ForEach-Object { $_.Name }) + @($provisioned | ForEach-Object { $_.DisplayName }) | Sort-Object -Unique)
    foreach ($name in $names) {
        $installed = @($packages | Where-Object { $_.Name -eq $name })
        $staged = @($provisioned | Where-Object { $_.DisplayName -eq $name })
        $blocked = ''
        if (@($installed | Where-Object { $_.NonRemovable -or $_.IsFramework -or $_.IsResourcePackage }).Count) {
            $blocked = 'Windows protects this package or other apps depend on it.'
        }
        $versions = @(@($installed | ForEach-Object { [string]$_.Version }) + @($staged | ForEach-Object { [string]$_.Version }) | Sort-Object -Unique)
        $details = @(
            (New-Detail 'Removal method' 'Windows Appx and provisioning'),
            (New-Detail 'Scope' $(if ($admin) { 'All users and new accounts' } else { 'Current user' })),
            (New-Detail 'Provisioned' $(if ($staged.Count) { 'Yes' } else { 'No' })),
            (New-Detail 'Package identities' ((@($installed | ForEach-Object { $_.PackageFullName }) + @($staged | ForEach-Object { $_.PackageName })) -join "`n"))
        )
        $apps.Add((New-App $name $name 'appx' ($versions -join ', ') $details $blocked))
    }
    foreach ($root in Get-DesktopRoots) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        foreach ($key in Get-ChildItem -LiteralPath $root) {
            $id = $root + '\' + $key.PSChildName
            $entry = Get-ItemProperty -LiteralPath $id
            if (-not $entry.DisplayName -or $entry.SystemComponent -eq 1 -or $entry.ParentKeyName) { continue }
            $blocked = ''
            $catalog = Get-CatalogId $entry
            $noUninstaller = -not $entry.UninstallString -and -not $entry.QuietUninstallString -and $entry.WindowsInstaller -ne 1
            if ($noUninstaller -and -not $catalog) {
                $blocked = 'This program has no registered uninstaller.'
            }
            $details = @(
                (New-Detail 'Publisher' $entry.Publisher),
                (New-Detail 'Removal method' $(if ($entry.WindowsInstaller -eq 1) { 'Windows Installer (MSI)' } else { 'Registered program uninstaller' })),
                (New-Detail 'Scope' $(if ($root -like '*HKEY_LOCAL_MACHINE*') { 'All users; administrator access required' } else { 'Current user' })),
                (New-Detail 'Install location' $entry.InstallLocation)
            )
            $app = New-App $entry.DisplayName $id 'desktop' $entry.DisplayVersion $details $blocked $catalog
            if ($noUninstaller -and $catalog) { $app.requires_permanent = $true }
            $apps.Add($app)
        }
    }
    foreach ($catalog in @('Microsoft.Edge', 'Microsoft.OneDrive')) {
        if (@($apps | Where-Object { $_.catalog_id -eq $catalog }).Count) { continue }
        $paths = @(Get-BundledPaths $catalog | Where-Object { Test-Path -LiteralPath (Join-Path $_.Root $_.Relative) })
        if (-not $paths.Count) { continue }
        $name = if ($catalog -eq 'Microsoft.Edge') { 'Microsoft Edge' } else { 'OneDrive' }
        $details = @((New-Detail 'Removal method' 'Bundled component file removal'), (New-Detail 'Component files' (($paths | ForEach-Object { Join-Path $_.Root $_.Relative }) -join "`n")))
        $app = New-App $name $catalog 'desktop' 'Bundled component files' $details '' $catalog
        $app.requires_permanent = $true
        $apps.Add($app)
    }
    if ($admin) {
        try {
            foreach ($feature in Get-WindowsOptionalFeature -Online) {
                if ([string]$feature.State -notin @('Enabled', 'Disabled')) { continue }
                $details = @((New-Detail 'Removal method' 'Windows optional feature servicing'), (New-Detail 'Permanent removal' 'Requests payload removal where Windows supports it; restoration may require installation media.'))
                $apps.Add((New-App $feature.FeatureName $feature.FeatureName 'feature' ([string]$feature.State) $details))
            }
        } catch { $warnings.Add("Could not read Windows features: $($_.Exception.Message)") }
        try {
            foreach ($capability in Get-WindowsCapability -Online) {
                if ([string]$capability.State -ne 'Installed') { continue }
                $details = @((New-Detail 'Removal method' 'Windows capability servicing'), (New-Detail 'Component files' 'Removing this capability deletes its installed payload. Permanent removal must be acknowledged.'))
                $apps.Add((New-App $capability.Name $capability.Name 'capability' ([string]$capability.State) $details))
            }
        } catch { $warnings.Add("Could not read Windows capabilities: $($_.Exception.Message)") }
    }
    return @{ apps = @($apps | Sort-Object name, kind, id); warnings = @($warnings.ToArray()) }
}

function Assert-DesktopIdentity([string]$Id) {
    foreach ($root in Get-DesktopRoots) {
        $prefix = $root + '\'
        if ($Id.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
            $leaf = $Id.Substring($prefix.Length)
            if ($leaf -and $leaf.IndexOfAny([char[]]'\/') -lt 0) { return }
        }
    }
    throw 'The desktop uninstall registration is invalid.'
}

function Split-UninstallCommand([string]$CommandLine) {
    $expanded = [Environment]::ExpandEnvironmentVariables($CommandLine).Trim()
    if ($expanded -match '^"([^"\r\n]+\.exe)"\s*(.*)$') {
        return @($matches[1], $matches[2])
    }
    if ($expanded -match '^([^"\r\n]+?\.exe)(?:\s+(.*))?$') {
        return @($matches[1].Trim(), [string]$matches[2])
    }
    throw 'This program does not register a supported executable uninstaller.'
}

function Invoke-Uninstaller([string]$Executable, [string]$Arguments) {
    if (-not [IO.Path]::IsPathRooted($Executable) -or -not (Test-Path -LiteralPath $Executable -PathType Leaf)) {
        throw 'The registered uninstaller executable could not be found.'
    }
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $Executable
    $start.Arguments = $Arguments
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($start)
    try {
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        $code = $process.ExitCode
        $text = ($stdout.GetAwaiter().GetResult() + "`n" + $stderr.GetAwaiter().GetResult()).Trim()
        if ($text) { $script:messages.Add($text) }
    } finally { $process.Dispose() }
    if ($code -notin @(0, 1641, 3010)) { throw "The program uninstaller returned exit code $code." }
    if ($code -in @(1641, 3010)) { $script:restart = $true }
}

function Assert-RemovalPath([string]$Path, [string]$Root) {
    $absoluteRoot = [IO.Path]::GetFullPath($Root).TrimEnd('\')
    $absolute = [IO.Path]::GetFullPath($Path)
    if (-not $absolute.StartsWith($absoluteRoot + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'The component path is outside its expected directory.'
    }
    $ancestor = $absolute
    while ($ancestor) {
        if (Test-Path -LiteralPath $ancestor) {
            $item = Get-Item -LiteralPath $ancestor -Force
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Component cleanup cannot follow a symbolic link or junction.' }
        }
        $ancestor = [IO.Path]::GetDirectoryName($ancestor)
    }
    if (Test-Path -LiteralPath $absolute -PathType Container) {
        $pending = [Collections.Generic.Stack[string]]::new()
        $pending.Push($absolute)
        while ($pending.Count) {
            foreach ($item in Get-ChildItem -LiteralPath $pending.Pop() -Force) {
                if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Component cleanup cannot follow a symbolic link or junction.' }
                if ($item.PSIsContainer) { $pending.Push($item.FullName) }
            }
        }
    }
    return $absolute
}

function Remove-ComponentPath([string]$Root, [string]$Relative) {
    if (-not $Root) { return }
    $path = Assert-RemovalPath (Join-Path $Root $Relative) $Root
    if (-not (Test-Path -LiteralPath $path)) { return }
    try { Remove-Item -LiteralPath $path -Recurse -Force }
    catch [UnauthorizedAccessException] {
        $ownershipArguments = @('/F', $path, '/A')
        $permission = '*S-1-5-32-544:F'
        if (Test-Path -LiteralPath $path -PathType Container) {
            $ownershipArguments += @('/R', '/D', 'Y')
            $permission = '*S-1-5-32-544:(OI)(CI)F'
        }
        $ownershipOutput = & (Join-Path $env:SystemRoot 'System32\takeown.exe') @ownershipArguments 2>&1 | Out-String
        if ($LASTEXITCODE -ne 0) { throw "Could not take ownership of component files: $ownershipOutput" }
        $permissionOutput = & (Join-Path $env:SystemRoot 'System32\icacls.exe') $path '/grant' $permission '/T' '/C' 2>&1 | Out-String
        if ($LASTEXITCODE -ne 0) { throw "Could not grant access to component files: $permissionOutput" }
        $path = Assert-RemovalPath $path $Root
        Remove-Item -LiteralPath $path -Recurse -Force
    }
    if (Test-Path -LiteralPath $path) { throw "Component files remain at $path." }
    $script:messages.Add("Permanently deleted component files: $path")
}

function Remove-BundledFiles([string]$Catalog) {
    Assert-Administrator
    foreach ($path in Get-BundledPaths $Catalog) {
        Remove-ComponentPath $path.Root $path.Relative
    }
}

function Remove-NativeApp($Request) {
    $script:messages = [Collections.Generic.List[string]]::new()
    $script:restart = $false
    try {
        $id = [string]$Request.id
        if (-not $id -or $id -match '[\x00-\x1f]') { throw 'Invalid app identity.' }
        $permanent = $Request.permanent -is [bool] -and $Request.permanent
        switch ($Request.kind) {
            'appx' {
                Assert-Administrator
                $packages = @(Get-AppxPackage -AllUsers -PackageTypeFilter Main, Bundle, Framework | Where-Object { $_.Name -eq $id })
                $staged = @(Get-AppxProvisionedPackage -Online | Where-Object { $_.DisplayName -eq $id })
                if (@($packages | Where-Object { $_.NonRemovable -or $_.IsFramework -or $_.IsResourcePackage }).Count) { throw 'Windows protects this package or other apps depend on it.' }
                if (-not $packages.Count -and -not $staged.Count) { throw 'This app is no longer installed or provisioned. Refresh the list.' }
                $errors = [Collections.Generic.List[string]]::new()
                foreach ($package in ($packages | Sort-Object @{ Expression = { $_.IsBundle }; Descending = $true })) {
                    try {
                        $stillInstalled = @(Get-AppxPackage -AllUsers -PackageTypeFilter Main, Bundle, Framework | Where-Object { $_.PackageFullName -eq $package.PackageFullName })
                        if (-not $stillInstalled.Count) { continue }
                        Remove-AppxPackage -Package $package.PackageFullName -AllUsers
                        $script:messages.Add("Removed for all users: $($package.PackageFullName)")
                    } catch { $errors.Add($_.Exception.Message) }
                }
                foreach ($package in $staged) {
                    try {
                        $result = Remove-AppxProvisionedPackage -Online -PackageName $package.PackageName -AllUsers
                        $script:restart = $script:restart -or [bool]$result.RestartNeeded
                        $script:messages.Add("Removed provisioning for new accounts: $($package.PackageName)")
                    } catch { $errors.Add($_.Exception.Message) }
                }
                $remaining = @(Get-AppxPackage -AllUsers | Where-Object { $_.Name -eq $id })
                $remainingStaged = @(Get-AppxProvisionedPackage -Online | Where-Object { $_.DisplayName -eq $id })
                if ($remaining.Count -or $remainingStaged.Count) { $errors.Add('Windows still reports installed or provisioned packages. A restart may be required.') }
                if ($errors.Count) { throw ($errors -join "`n") }
            }
            'desktop' {
                if ($id -in @('Microsoft.Edge', 'Microsoft.OneDrive')) {
                    if (-not $permanent) { throw 'Bundled component files require permanent removal to be acknowledged first.' }
                    $paths = @(Get-BundledPaths $id | Where-Object { Test-Path -LiteralPath (Join-Path $_.Root $_.Relative) })
                    if (-not $paths.Count) { throw 'These bundled component files are no longer present. Refresh the list.' }
                    Remove-BundledFiles $id
                    break
                }
                Assert-DesktopIdentity $id
                if ($id -like '*HKEY_LOCAL_MACHINE*') { Assert-Administrator }
                $entry = Get-ItemProperty -LiteralPath $id
                if ($entry.SystemComponent -eq 1 -or $entry.ParentKeyName) { throw 'This is a protected component or a child of another installer.' }
                if ([string]$entry.DisplayVersion -ne [string]$Request.version) { throw 'This program changed since the list was loaded. Refresh before removing it.' }
                $catalog = Get-CatalogId $entry
                if ($permanent -and $catalog) { Assert-Administrator }
                $uninstallError = $null
                try {
                    if ($entry.WindowsInstaller -eq 1) {
                        $product = Split-Path $id -Leaf
                        if ($product -notmatch '^\{[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}\}$') { throw 'Invalid MSI product code.' }
                        Invoke-Uninstaller (Join-Path $env:SystemRoot 'System32\msiexec.exe') "/x $product /passive /norestart"
                    } else {
                        $line = [string]$entry.QuietUninstallString
                        if (-not $line) { $line = [string]$entry.UninstallString }
                        $parts = Split-UninstallCommand $line
                        Invoke-Uninstaller $parts[0] $parts[1]
                    }
                    $script:messages.Add('Completed the registered program uninstaller.')
                } catch { $uninstallError = $_.Exception.Message }
                if ($permanent -and $catalog) {
                    Remove-BundledFiles $catalog
                    if (Test-Path -LiteralPath $id) { Remove-Item -LiteralPath $id -Recurse -Force }
                    if ($uninstallError) { $script:messages.Add("The registered uninstaller failed; component files were removed permanently. $uninstallError") }
                } else {
                    if ($uninstallError) { throw $uninstallError }
                    if ((Test-Path -LiteralPath $id) -and -not $script:restart) { throw 'The program is still registered after its uninstaller finished. Complete any installer prompts, then refresh.' }
                }
            }
            'feature' {
                Assert-Administrator
                $feature = @(Get-WindowsOptionalFeature -Online | Where-Object { $_.FeatureName -eq $id })
                if ($feature.Count -ne 1) { throw 'This optional feature is no longer available. Refresh the list.' }
                if ([string]$feature[0].State -eq 'Disabled' -and -not $permanent) { throw 'This feature is already disabled. Permanent removal must be acknowledged to request payload removal.' }
                $parameters = @{ Online = $true; FeatureName = $id; NoRestart = $true }
                if ($permanent) { $parameters.Remove = $true }
                $result = Disable-WindowsOptionalFeature @parameters
                $script:restart = [bool]$result.RestartNeeded
                $script:messages.Add('Disabled the selected Windows optional feature.')
                if ($permanent) { $script:messages.Add('Requested component payload removal. Windows client editions may retain the payload for recovery; no shared component-store files were deleted.') }
            }
            'capability' {
                if (-not $permanent) { throw 'Removing this capability deletes component files. Enable permanent removal and acknowledge the warning first.' }
                Assert-Administrator
                $capability = @(Get-WindowsCapability -Online | Where-Object { $_.Name -eq $id -and [string]$_.State -eq 'Installed' })
                if ($capability.Count -ne 1) { throw 'This capability is no longer installed. Refresh the list.' }
                $result = Remove-WindowsCapability -Online -Name $id
                $script:restart = [bool]$result.RestartNeeded
                $script:messages.Add('Removed the Windows capability and its installed payload. Reinstallation requires Windows Update or installation media.')
            }
            default { throw 'Unsupported app identity type.' }
        }
        if ($script:restart) { $script:messages.Add('Restart Windows to finish removal.') }
        return @{ success = $true; output = ($script:messages -join "`n"); restart_required = $script:restart }
    } catch {
        $script:messages.Add($_.Exception.Message)
        return @{ success = $false; output = ($script:messages -join "`n"); restart_required = $script:restart }
    }
}

try {
    $request = [Console]::In.ReadToEnd() | ConvertFrom-Json
    switch ($request.action) {
        'list' { $result = Get-NativeInventory }
        'remove' { $result = Remove-NativeApp $request }
        default { throw 'Unknown app servicing request.' }
    }
    [Console]::WriteLine((ConvertTo-Json -InputObject $result -Depth 8 -Compress))
} catch {
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
}
