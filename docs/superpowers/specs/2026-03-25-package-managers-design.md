# bflow Package Manager Distribution

**Date:** 2026-03-25
**Status:** Approved

## Overview

Distribute bflow via Homebrew (macOS) and Chocolatey (Windows) so users can install with a single command. Both package managers pull pre-built binaries from GitHub Releases — no compilation required on the user's machine.

CI automation updates both package managers on each tagged release (no manual maintenance).

## Homebrew (macOS)

### Tap repository

Repo: `Beans-BV/homebrew-tap` (already created)

Formula file: `Formula/bflow.rb` (already created with placeholder SHAs)

**User install:**
```bash
brew tap Beans-BV/tap
brew install bflow
```

### Formula structure

The formula downloads the correct pre-built binary (aarch64 or x86_64) from the GitHub Release. No compilation step.

```ruby
class Bflow < Formula
  desc "Beans GitFlow - customized gitflow workflow CLI"
  homepage "https://github.com/Beans-BV/beans-gitflow"
  version "VERSION"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Beans-BV/beans-gitflow/releases/download/v#{version}/bflow-macos-aarch64"
      sha256 "SHA256_ARM"
    else
      url "https://github.com/Beans-BV/beans-gitflow/releases/download/v#{version}/bflow-macos-x86_64"
      sha256 "SHA256_X86"
    end
  end

  def install
    bin.install stable.url.split("/").last => "bflow"
  end
end
```

### CI automation (update-homebrew job)

Runs after `create-release` job. Steps:

1. Download both macOS binaries from the release
2. Compute SHA256 for each
3. Clone `Beans-BV/homebrew-tap` using `HOMEBREW_TAP_TOKEN`
4. Update `Formula/bflow.rb` using `sed`:
   - Replace the `version "..."` line with the new version
   - Replace the `sha256 "..."` lines (first occurrence = ARM, second = x86)
5. Commit and push

**Auth:** `HOMEBREW_TAP_TOKEN` secret (fine-grained PAT with Contents read/write on `homebrew-tap` repo). Already configured.

## Chocolatey (Windows)

### Package files

Located in `packaging/chocolatey/` within the `beans-gitflow` repo:

- `bflow.nuspec` — package metadata (id, version, description, project URL, license)
- `tools/chocolateyinstall.ps1` — downloads the Windows binary from GitHub Release, installs to Chocolatey bin
- `tools/chocolateyuninstall.ps1` — removes the binary

**User install:**
```powershell
choco install bflow
```

### nuspec

```xml
<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://schemas.chocolatey.org/2010/07">
  <metadata>
    <id>bflow</id>
    <version>VERSION</version>
    <title>bflow - Beans GitFlow CLI</title>
    <authors>Beans BV</authors>
    <projectUrl>https://github.com/Beans-BV/beans-gitflow</projectUrl>
    <licenseUrl>https://github.com/Beans-BV/beans-gitflow/blob/main/LICENSE</licenseUrl>
    <requireLicenseAcceptance>false</requireLicenseAcceptance>
    <description>A cross-platform CLI tool that implements the Beans customized gitflow workflow.</description>
    <tags>git gitflow cli workflow branching</tags>
  </metadata>
  <files>
    <file src="tools\**" target="tools" />
  </files>
</package>
```

### chocolateyinstall.ps1

```powershell
$ErrorActionPreference = 'Stop'
$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$version = $env:chocolateyPackageVersion
$url = "https://github.com/Beans-BV/beans-gitflow/releases/download/v${version}/bflow-windows-x86_64.exe"
$checksum = 'CHECKSUM'

Get-ChocolateyWebFile -PackageName 'bflow' `
  -FileFullPath "$toolsDir\bflow.exe" `
  -Url64bit $url `
  -Checksum64 $checksum `
  -ChecksumType64 'sha256'
```

### chocolateyuninstall.ps1

```powershell
$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
Remove-Item "$toolsDir\bflow.exe" -Force -ErrorAction SilentlyContinue
```

### CI automation (publish-chocolatey job)

Runs after `create-release` job on `windows-latest`. Steps:

1. Checkout the repo (to access `packaging/chocolatey/`)
2. Download the Windows binary from the release
3. Compute SHA256 checksum
4. Update `chocolateyinstall.ps1` checksum using PowerShell string replacement
5. Run `choco pack packaging/chocolatey/bflow.nuspec --version $VERSION` (version via CLI flag, no nuspec editing needed)
6. Run `choco push` with `CHOCOLATEY_API_KEY` secret

**Note:** First-time Chocolatey community submission goes through moderation review (can take days). Subsequent updates are faster.

**Auth:** `CHOCOLATEY_API_KEY` secret. Already configured.

## CI Workflow Update

The existing `ci.yml` gets two new jobs added after `create-release`:

```
tag push (v*) → test → release (build binaries) → create-release (GitHub Release)
                                                        ├→ update-homebrew
                                                        └→ publish-chocolatey
```

Both new jobs use `needs: create-release` to ensure the GitHub Release with binaries exists before they run.

## README Update

Update the Installation section to show all three install methods:

```markdown
### Homebrew (macOS)
brew tap Beans-BV/tap
brew install bflow

### Chocolatey (Windows)
choco install bflow

### From source
cargo install --path .
```

## Secrets Required

| Secret | Repo | Purpose | Status |
|--------|------|---------|--------|
| `HOMEBREW_TAP_TOKEN` | beans-gitflow | Push formula updates to homebrew-tap | Configured |
| `CHOCOLATEY_API_KEY` | beans-gitflow | Push packages to Chocolatey community repo | Configured |
