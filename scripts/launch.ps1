# Windows PowerShell 5.1+, included with supported desktop Windows.
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
$cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
$env:PATH = "$(Join-Path $cargoHome 'bin');$env:PATH"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

function Install-DownloadedTool($Url, $Arguments) {
    $temporary = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString() + '.exe')
    try {
        Invoke-WebRequest -UseBasicParsing -Uri $Url -OutFile $temporary
        $process = Start-Process -FilePath $temporary -ArgumentList $Arguments -Wait -PassThru
        if ($process.ExitCode -eq 3010) {
            throw 'Installation requires a Windows restart. Restart, then double-click windows_launcher.cmd again.'
        }
        if ($process.ExitCode -ne 0) { throw "Installer failed with exit code $($process.ExitCode)." }
    } finally {
        Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
    }
}

try {
    $architecture = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
    $vcComponent = if ($architecture -eq 'ARM64') { 'Microsoft.VisualStudio.Component.VC.Tools.ARM64' } else { 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64' }
    # Rust's MSVC toolchain discovers installed Visual Studio tools itself.
    $rustInfo = if (Get-Command rustc -ErrorAction SilentlyContinue) { & rustc -vV 2>$null } else { '' }
    if (($rustInfo -join "`n") -notmatch 'host: .*windows-gnu') {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
        $hasTools = $false
        if (Test-Path $vswhere) {
            $hasTools = [bool](& $vswhere -latest -products '*' -requires $vcComponent -property installationPath)
        }
        if (!$hasTools) {
            Write-Host 'Installing Microsoft C++ Build Tools and Windows SDK. Accept the administrator prompt; this can take several minutes.'
            Install-DownloadedTool 'https://aka.ms/vs/17/release/vs_buildtools.exe' "--passive --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --add $vcComponent"
        }
    }
    if (!(Get-Command cargo -ErrorAction SilentlyContinue) -or !(Get-Command rustc -ErrorAction SilentlyContinue)) {
        $target = switch ($architecture) {
            'ARM64' { 'aarch64-pc-windows-msvc' }
            'X86' { 'i686-pc-windows-msvc' }
            default { 'x86_64-pc-windows-msvc' }
        }
        Write-Host 'Installing Rust with the official rustup installer.'
        Install-DownloadedTool "https://static.rust-lang.org/rustup/dist/$target/rustup-init.exe" '-y --profile minimal'
    }
    $versionText = & rustc --version
    if ($LASTEXITCODE -ne 0) { throw 'Rust is installed but unavailable. Repair your Rust installation and rerun.' }
    if ($versionText -notmatch 'rustc (\d+)\.(\d+)') { throw "Cannot read Rust version: $versionText" }
    if ([int]$Matches[1] -lt 1 -or ([int]$Matches[1] -eq 1 -and [int]$Matches[2] -lt 70)) {
        if (!(Get-Command rustup -ErrorAction SilentlyContinue)) { throw 'Update your Rust installation to 1.70 or newer and rerun.' }
        & rustup toolchain install stable --profile minimal
        if ($LASTEXITCODE -ne 0) { throw 'Rust update failed.' }
        $env:RUSTUP_TOOLCHAIN = 'stable'
    }
    & cargo build --release --offline
    if ($LASTEXITCODE -ne 0) { throw 'Build failed. See the compiler message above.' }
    & .\target\release\chess-ai.exe --open @args
    exit $LASTEXITCODE
} catch {
    Write-Host $_ -ForegroundColor Red
    exit 1
}
