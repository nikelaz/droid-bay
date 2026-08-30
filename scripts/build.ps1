$ErrorActionPreference = "Stop"

$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Dist = Join-Path $Root "dist"
$Platforms = @(
	"linux/amd64",
	"linux/arm64",
	"darwin/amd64",
	"darwin/arm64",
	"windows/amd64"
)

if (Test-Path $Dist) {
	Remove-Item -Recurse -Force $Dist
}
New-Item -ItemType Directory -Force -Path (Join-Path $Dist "agents") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Dist "skills") | Out-Null

Get-ChildItem -Directory (Join-Path $Root "agents") | ForEach-Object {
	$agentDir = $_.FullName
	$name = $_.Name
	if (-not (Test-Path (Join-Path $agentDir "go.mod"))) {
		return
	}

	foreach ($platform in $Platforms) {
		$os, $arch = $platform -split "/"
		$outDir = Join-Path $Dist "agents" $name "$os-$arch"
		New-Item -ItemType Directory -Force -Path $outDir | Out-Null
		$binary = $name
		if ($os -eq "windows") {
			$binary = "$name.exe"
		}

		$previous = @{}
		foreach ($variable in "GOOS", "GOARCH", "CGO_ENABLED") {
			$previous[$variable] = [Environment]::GetEnvironmentVariable($variable)
		}

		Push-Location $agentDir
		try {
			$env:GOOS = $os
			$env:GOARCH = $arch
			$env:CGO_ENABLED = "0"
			go build -trimpath -o (Join-Path $outDir $binary) .
		} finally {
			foreach ($variable in "GOOS", "GOARCH", "CGO_ENABLED") {
				if ($null -ne $previous[$variable]) {
					[Environment]::SetEnvironmentVariable($variable, $previous[$variable])
				} else {
					Remove-Item "Env:$variable" -ErrorAction SilentlyContinue
				}
			}
			Pop-Location
		}
		Write-Host "built agents/$name/$os-$arch/$binary"
	}
}

$skills = Join-Path $Root "skills"
if (Test-Path $skills) {
	Get-ChildItem $skills | ForEach-Object {
		Copy-Item $_.FullName (Join-Path $Dist "skills") -Recurse
	}
}

Write-Host "release ready at $Dist"
