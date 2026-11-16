# Downloads libmpv (the MPV engine: FFmpeg + libplacebo + hardware decoding) into vendor/mpv.
# Build: shinchiro/mpv-winbuild-cmake (static, self-contained libmpv-2.dll, GPL).
#   powershell -ExecutionPolicy Bypass -File scripts/fetch-libmpv.ps1 [-Force]
param([switch]$Force)
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# Pinned release (bump both values to update libmpv).
$Tag = "20261111"
$Asset = "mpv-dev-x86_64-20261111-git-bcde6b93cc.7z"

$root = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $root "vendor\mpv"
$dll = Join-Path $dest "libmpv-2.dll"
if ((Test-Path $dll) -and -not $Force) { Write-Host "libmpv: already present ($dll)"; exit 0 }

New-Item -ItemType Directory -Force $dest | Out-Null
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("videk-mpv-" + [Guid]::NewGuid())
New-Item -ItemType Directory $tmp | Out-Null
try {
  $url = "https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/$Tag/$Asset"
  Write-Host "libmpv: downloading $url"
  $archive = Join-Path $tmp $Asset
  Invoke-WebRequest -Uri $url -OutFile $archive -UseBasicParsing
  # Windows 10/11 tar.exe (bsdtar/libarchive) reads .7z.
  tar -xf $archive -C $tmp
  if ($LASTEXITCODE -ne 0) { throw "tar failed to extract $Asset" }
  $found = Get-ChildItem -Path $tmp -Recurse -Filter "libmpv-2.dll" | Select-Object -First 1
  if (-not $found) { throw "libmpv-2.dll not found in $Asset" }
  Copy-Item $found.FullName $dll -Force
  Set-Content -Path (Join-Path $dest "VERSION.txt") -Value "$Tag $Asset" -Encoding utf8
  Write-Host "libmpv: ok -> $dll"
} finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
