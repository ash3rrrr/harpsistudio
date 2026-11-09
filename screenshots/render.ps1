$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$root = Split-Path -Parent $PSScriptRoot
$raw = Join-Path $PSScriptRoot "raw"
$exe = Join-Path $root "target\release\harpsistudio.exe"

if (Test-Path $raw) {
    Remove-Item $raw -Recurse -Force
}
New-Item -ItemType Directory -Path $raw | Out-Null

$env:HARPSI_CAPTURE_DIR = $raw

function Invoke-CaptureRun([int]$start, [int]$end, [int]$expected) {
    $env:HARPSI_CAPTURE_START = "$start"
    $env:HARPSI_CAPTURE_END = "$end"
    $proc = Start-Process -FilePath $exe -WorkingDirectory $root -PassThru
    Wait-Process -Id $proc.Id -Timeout 90 -ErrorAction SilentlyContinue
    if (-not $proc.HasExited) {
        Stop-Process -Id $proc.Id -Force
        $log = Join-Path $raw "capture.log"
        if (Test-Path $log) { Get-Content $log | Write-Output }
        throw ("Capture run {0}-{1} exceeded 90 seconds" -f $start, $end)
    }

    $count = (Get-ChildItem (Join-Path $raw "*.bmp") | Measure-Object).Count
    if ($count -ne $expected) {
        $log = Join-Path $raw "capture.log"
        if (Test-Path $log) { Get-Content $log | Write-Output }
        throw ("Capture run {0}-{1}: expected {2} total bitmaps, found {3}" -f $start, $end, $expected, $count)
    }
}

Invoke-CaptureRun 0 5 5
Invoke-CaptureRun 5 8 8

$bitmaps = Get-ChildItem (Join-Path $raw "*.bmp") | Sort-Object Name
if ($bitmaps.Count -ne 8) {
    throw ("Expected 8 captures, found {0}" -f $bitmaps.Count)
}

foreach ($file in $bitmaps) {
    $bitmap = [System.Drawing.Bitmap]::FromFile($file.FullName)
    $target = Join-Path $PSScriptRoot ($file.BaseName + ".png")
    $bitmap.Save($target, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    $size = (Get-Item $target).Length
    Write-Output ("RENDER={0} BYTES={1}" -f $target, $size)
}

$errors = Get-ChildItem (Join-Path $raw "*.error.txt") -ErrorAction SilentlyContinue
if ($errors) {
    Get-Content $errors | Write-Output
    throw "One or more screenshots failed to save"
}
