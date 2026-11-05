$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location $Root
$Subscriber = $null
$OutputFile = Join-Path ([System.IO.Path]::GetTempPath()) ("d2s-sub-" + [guid]::NewGuid().ToString("N") + ".log")
try {
    & cargo build -p d2s --examples
    if ($LASTEXITCODE -ne 0) { throw "Cargo build failed" }

    $SubscriberExe = Join-Path $Root "target\debug\examples\subscriber.exe"
    $PublisherExe = Join-Path $Root "target\debug\examples\publisher.exe"

    $Subscriber = Start-Process -FilePath $SubscriberExe -ArgumentList @("10", "3") -PassThru -RedirectStandardOutput $OutputFile
    Start-Sleep -Seconds 2

    & $PublisherExe "10" "3"
    if ($LASTEXITCODE -ne 0) { throw "Publisher failed (exit code $LASTEXITCODE)" }

    if (!$Subscriber.WaitForExit(20000)) {
        $Subscriber.Kill()
        throw "Subscriber did not exit after receiving messages"
    }
    if ($Subscriber.ExitCode -ne 0) { throw "Subscriber failed (exit code $($Subscriber.ExitCode))" }

    $Messages = @(Get-Content -LiteralPath $OutputFile | Where-Object { $_ -match 'RECEIVED \[' })
    if ($Messages.Count -ne 3) {
        throw "Expected 3 received messages, got $($Messages.Count)"
    }
    Write-Host "D2S Phase 0 Windows: PASS (three typed DDS messages)"
} finally {
    if ($null -ne $Subscriber) {
        if (!$Subscriber.HasExited) { $Subscriber.Kill() }
        $Subscriber.Dispose()
    }
    if (Test-Path -LiteralPath $OutputFile) { Remove-Item -LiteralPath $OutputFile -Force }
    Pop-Location
}
