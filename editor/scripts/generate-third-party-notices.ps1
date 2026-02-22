$ErrorActionPreference = 'Stop'

$projectRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$outputPath = Join-Path $projectRoot 'app/assets/THIRD-PARTY-NOTICES.txt'
$tempDir = Join-Path $env:TEMP ("symi-third-party-" + [guid]::NewGuid().ToString())

New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

try {
  $nodeRawOutputPath = Join-Path $tempDir 'node-licenses.json'
  $nodeOutputPath = Join-Path $tempDir 'node-licenses.txt'
  $rustOutputPath = Join-Path $tempDir 'rust-licenses.txt'
  $templatePath = Join-Path $tempDir 'about.hbs'
  $aboutConfigPath = Join-Path $projectRoot 'src-tauri/about.toml'

  $templateContent = @'
{{#each licenses}}
========================================
License: {{name}}
========================================

Used by crates:
{{#each used_by}}
- {{crate.name}} {{crate.version}}
{{/each}}

{{text}}

{{/each}}
'@
  Set-Content -Path $templatePath -Value $templateContent -Encoding UTF8

  if (-not (Test-Path $aboutConfigPath)) {
    throw "Missing cargo-about config: $aboutConfigPath"
  }

  Push-Location $projectRoot
  try {
    bunx --bun license-checker --production --json | Out-File -FilePath $nodeRawOutputPath -Encoding UTF8
  }
  finally {
    Pop-Location
  }

  $nodeLicenses = Get-Content -Path $nodeRawOutputPath -Raw | ConvertFrom-Json
  $nodeLines = New-Object System.Collections.Generic.List[string]
  $nodeLicenseProperties = $nodeLicenses.PSObject.Properties | Sort-Object Name

  foreach ($property in $nodeLicenseProperties) {
    $packageName = $property.Name
    $entry = $property.Value

    $licenseValue = ''
    if ($entry.licenses -is [System.Array]) {
      $licenseValue = ($entry.licenses -join ', ')
    }
    else {
      $licenseValue = [string]$entry.licenses
    }

    $repositoryValue = ''
    if ($entry.repository) {
      $repositoryValue = [string]$entry.repository
    }
    elseif ($entry.url) {
      $repositoryValue = [string]$entry.url
    }

    $nodeLines.Add("- $packageName")
    $nodeLines.Add("  license: $licenseValue")
    if ($repositoryValue) {
      $nodeLines.Add("  repository: $repositoryValue")
    }
    $nodeLines.Add('')
  }

  Set-Content -Path $nodeOutputPath -Value $nodeLines -Encoding UTF8

  if (-not (Get-Command cargo-about -ErrorAction SilentlyContinue)) {
    cargo install cargo-about --locked
  }

  $tauriRoot = Join-Path $projectRoot 'src-tauri'
  Push-Location $tauriRoot
  try {
    cargo about -L error generate --manifest-path (Join-Path $tauriRoot 'Cargo.toml') -c $aboutConfigPath $templatePath --output-file $rustOutputPath
  }
  finally {
    Pop-Location
  }

  $header = @(
    'THIRD-PARTY NOTICES'
    ('Generated on: ' + (Get-Date).ToString('u'))
    ''
    '========================'
    'JavaScript Dependencies'
    '========================'
    ''
  )

  $middle = @(
    ''
    '=================='
    'Rust Dependencies'
    '=================='
    ''
  )

  $footer = @(
    ''
    'End of THIRD-PARTY NOTICES'
  )

  $finalContent = @()
  $finalContent += $header
  $finalContent += Get-Content -Path $nodeOutputPath
  $finalContent += $middle
  $finalContent += Get-Content -Path $rustOutputPath
  $finalContent += $footer

  $outputDir = Split-Path -Parent $outputPath
  New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
  Set-Content -Path $outputPath -Value $finalContent -Encoding UTF8

  Write-Host "Generated: $outputPath"
}
finally {
  if (Test-Path $tempDir) {
    Remove-Item -Path $tempDir -Force -Recurse
  }
}
