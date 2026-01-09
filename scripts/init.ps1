function p {
    # If the first argument is 'j' (jump), handle it specially
    if ($args[0] -eq "j") {
        $path = $args[1]
        if (-not $path) {
            Write-Host "Usage: p j <path>"
            return
        }

        $tmpFile = [System.IO.Path]::GetTempFileName()
        
        $env:PAVIDI_OUTPUT = $tmpFile
        # Pass 'j' and the path explicitly
        & p.exe j $path
        $env:PAVIDI_OUTPUT = $null

        if ($LASTEXITCODE -eq 0) {
            $targetDir = Get-Content $tmpFile
            if (Test-Path $targetDir) {
                Set-Location $targetDir
            }
        }
        
        Remove-Item $tmpFile
    } else {
        # For all other commands, pass all arguments through directly
        & p.exe @args
    }
}
