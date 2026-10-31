# Élague le dossier target des artefacts des crates du dépôt (optimisation D-2).
#
# Quand le lancer : après chaque montée de version (`version` de [workspace.package] dans le
# Cargo.toml racine) et avant une vérification complète. Chaque version recompile toutes les
# crates du dépôt sous un nouveau nom dans target/debug/deps sans effacer les anciennes, et le
# disque se remplit. Ne jamais supprimer de rlib à la main : ce script passe par `cargo clean -p`,
# qui retire les artefacts de chaque crate du dépôt, toutes versions comprises, sans toucher aux
# dépendances (elles ne seront pas recompilées).
#
#   powershell -ExecutionPolicy Bypass -File outils\elaguer-target.ps1
#   powershell -ExecutionPolicy Bypass -File outils\elaguer-target.ps1 -Release -Incremental
#
# -Release     élague aussi target/release.
# -Incremental vide aussi target/debug/incremental (la prochaine compilation sera plus longue).
param(
    [switch]$Release,
    [switch]$Incremental
)

$ErrorActionPreference = 'Stop'

function Get-Taille([string]$Dossier) {
    $somme = (Get-ChildItem -LiteralPath $Dossier -Recurse -File -Force -ErrorAction SilentlyContinue |
        Measure-Object -Property Length -Sum).Sum
    if ($null -eq $somme) { return [int64]0 }
    return [int64]$somme
}

function Format-Taille([int64]$Octets) {
    if ([math]::Abs($Octets) -ge 1GB) { return '{0:N2} Go' -f ($Octets / 1GB) }
    return '{0:N0} Mo' -f ($Octets / 1MB)
}

# Commande cargo : la sortie d'erreur (où cargo écrit « Removed … ») ne doit pas être prise pour
# une erreur PowerShell ; seul le code de retour compte.
function Invoke-Cargo([string[]]$Arguments) {
    $ErrorActionPreference = 'Continue'
    & cargo @Arguments
    if ($LASTEXITCODE -ne 0) { throw "cargo $($Arguments -join ' ') a échoué (code $LASTEXITCODE)." }
}

try {
    Set-Location -LiteralPath (Split-Path -Parent $PSScriptRoot)

    # Paquets de l'espace de travail et dossier target, lus dans cargo metadata (pas de liste en dur).
    $json = & cargo metadata --offline --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw "cargo metadata a échoué (code $LASTEXITCODE)." }
    $metadata = ($json -join "`n") | ConvertFrom-Json
    $paquets = @($metadata.packages | ForEach-Object { $_.name })
    $target = $metadata.target_directory

    if (-not (Test-Path -LiteralPath $target)) {
        Write-Host "Pas de dossier $target : rien à élaguer."
        exit 0
    }

    $avant = Get-Taille $target
    Write-Host "Taille de target avant : $(Format-Taille $avant)"
    Write-Host "Paquets du dépôt : $($paquets -join ', ')"

    $argumentsPaquets = @($paquets | ForEach-Object { '-p'; $_ })
    Write-Host 'Élagage du profil dev…'
    Invoke-Cargo (@('clean', '--offline') + $argumentsPaquets)
    if ($Release) {
        Write-Host 'Élagage du profil release…'
        Invoke-Cargo (@('clean', '--offline', '--release') + $argumentsPaquets)
    }

    if ($Incremental) {
        # Pas de $incremental ici : PowerShell ignore la casse et écraserait le paramètre.
        $dossierIncremental = Join-Path $target 'debug\incremental'
        if (Test-Path -LiteralPath $dossierIncremental) {
            Write-Host "Vidage de $dossierIncremental…"
            Get-ChildItem -LiteralPath $dossierIncremental -Force | Remove-Item -Recurse -Force
        }
    }

    $apres = Get-Taille $target
    Write-Host "Taille de target après : $(Format-Taille $apres)"
    Write-Host "Place gagnée : $(Format-Taille ($avant - $apres))"
}
catch {
    Write-Host "Élagage interrompu : $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}
