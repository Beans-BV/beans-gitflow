$ErrorActionPreference = 'Stop'
$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$version = $env:chocolateyPackageVersion
$url = "https://github.com/Beans-BV/beans-gitflow/releases/download/v${version}/bflow-windows-x86_64.exe"
$checksum = '__CHECKSUM__'

Get-ChocolateyWebFile -PackageName 'bflow' `
  -FileFullPath "$toolsDir\bflow.exe" `
  -Url64bit $url `
  -Checksum64 $checksum `
  -ChecksumType64 'sha256'
