param([string]$Profile = "release")

$flag = if ($Profile -eq "release") { "--release" } else { "" }
cargo build $flag
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$dir = "target\$Profile"
Copy-Item "$dir\grannyultimatefixes.dll" "$dir\grannyultimatefixes.asi" -Force
Write-Host "output: $dir\grannyultimatefixes.asi"
