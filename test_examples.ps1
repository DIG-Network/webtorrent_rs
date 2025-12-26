#!/usr/bin/env pwsh
# Script to test all examples with timeouts and expected output verification

$ErrorActionPreference = "Continue"
$projectDir = "C:\Users\micha\workspace\dig_network\webtorrent_rs"
$outputFile = Join-Path $projectDir "test_examples_results.txt"
Set-Location $projectDir

# Redirect all output to both console and file
$null = Start-Transcript -Path $outputFile

$cargoExe = "$env:USERPROFILE\.cargo\bin\cargo.exe"
$timeoutSeconds = 30

# Get all example files (exclude utility files)
$exampleFiles = Get-ChildItem examples\*.rs -Exclude test_utils.rs,test_runner.rs | 
    Select-Object -ExpandProperty BaseName | 
    Sort-Object

Write-Host "Found $($exampleFiles.Count) examples to test`n"

# Step 1: Check compilation for all examples
Write-Host "=== Step 1: Checking compilation ===" -ForegroundColor Cyan
$compileErrors = @()
$compiled = @()

foreach ($ex in $exampleFiles) {
    Write-Host -NoNewline "  Compiling $ex ... "
    $output = & $cargoExe check --example $ex 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "OK" -ForegroundColor Green
        $compiled += $ex
    } else {
        Write-Host "ERROR" -ForegroundColor Red
        $compileErrors += $ex
        $output | Select-String -Pattern "error\[E\d+\]" | Select-Object -First 1 | ForEach-Object {
            Write-Host "    $_" -ForegroundColor Yellow
        }
    }
}

Write-Host "`nCompilation Summary: $($compiled.Count) OK, $($compileErrors.Count) errors`n"

if ($compileErrors.Count -gt 0) {
    Write-Host "Examples with compilation errors:" -ForegroundColor Red
    $compileErrors | ForEach-Object { Write-Host "  $_" -ForegroundColor Yellow }
    Write-Host ""
}

# Step 2: Run compiled examples with timeout
Write-Host "=== Step 2: Running examples (timeout: ${timeoutSeconds}s) ===" -ForegroundColor Cyan
$passed = @()
$failed = @()
$timeouts = @()

foreach ($ex in $compiled) {
    Write-Host -NoNewline "  Running $ex ... "
    
    $job = Start-Job -ScriptBlock {
        param($cargo, $example)
        Set-Location $using:projectDir
        & $cargo run --example $example 2>&1
    } -ArgumentList $cargoExe, $ex
    
    $result = $job | Wait-Job -Timeout $timeoutSeconds
    
    if ($result) {
        $output = $job | Receive-Job
        $job | Remove-Job
        
        # Check for compilation/runtime errors
        if ($output -match "error\[") {
            Write-Host "ERROR" -ForegroundColor Red
            $failed += $ex
            $output | Select-String -Pattern "error\[" | Select-Object -First 1 | ForEach-Object {
                Write-Host "    $_" -ForegroundColor Yellow
            }
        }
        # Check for expected "PASSED" output
        elseif ($output -match "===.*PASSED ===") {
            Write-Host "PASS" -ForegroundColor Green
            $passed += $ex
        }
        # Check for panics or other failures
        elseif ($output -match "panicked|thread.*panicked|fatal") {
            Write-Host "PANIC" -ForegroundColor Red
            $failed += $ex
        }
        else {
            Write-Host "UNKNOWN" -ForegroundColor Yellow
            Write-Host "    (No PASSED marker found)" -ForegroundColor Yellow
            $failed += $ex
        }
    } else {
        $job | Stop-Job
        $job | Remove-Job
        Write-Host "TIMEOUT" -ForegroundColor Magenta
        $timeouts += $ex
    }
}

# Final summary
Write-Host "`n=== Final Summary ===" -ForegroundColor Cyan
Write-Host "Total examples: $($exampleFiles.Count)"
Write-Host "Compiled: $($compiled.Count)" -ForegroundColor Green
Write-Host "Passed: $($passed.Count)" -ForegroundColor Green
Write-Host "Failed: $($failed.Count)" -ForegroundColor Red
Write-Host "Timeouts: $($timeouts.Count)" -ForegroundColor Magenta
Write-Host "Compile errors: $($compileErrors.Count)" -ForegroundColor Red

if ($failed.Count -gt 0) {
    Write-Host "`nFailed examples:" -ForegroundColor Red
    $failed | ForEach-Object { Write-Host "  $_" -ForegroundColor Yellow }
}

if ($timeouts.Count -gt 0) {
    Write-Host "`nTimed out examples (may need longer timeout):" -ForegroundColor Magenta
    $timeouts | ForEach-Object { Write-Host "  $_" -ForegroundColor Yellow }
}

if ($compileErrors.Count -gt 0) {
    Write-Host "`nCompilation errors:" -ForegroundColor Red
    $compileErrors | ForEach-Object { Write-Host "  $_" -ForegroundColor Yellow }
}

# Exit with error code if any failures
Stop-Transcript
if (($compileErrors.Count + $failed.Count) -gt 0) {
    exit 1
} else {
    exit 0
}

