$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
Remove-Item "$toolsDir\bflow.exe" -Force -ErrorAction SilentlyContinue
