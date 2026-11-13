# Instalador de Logic_F-4:13 para Windows:
#   irm https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.ps1 | iex
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.ps1))) -Si
param([switch]$Si, [string]$Dir = "$env:LOCALAPPDATA\Programs\LogicF4")

$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
# La barra de Invoke-WebRequest hace la descarga mucho mas lenta en Windows PowerShell 5
if ($PSVersionTable.PSVersion.Major -lt 6) { $ProgressPreference = 'SilentlyContinue' }

$Repo = 'VstevenMata/New-Logic-IDE'
$Nombre = 'logicf4-ide'
$Version = '1.0.0'
$Acceso = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Logic F-4.13.lnk"

function Info($m) { Write-Host 'info: ' -ForegroundColor Green -NoNewline; Write-Host $m }
function Aviso($m) { Write-Host 'aviso: ' -ForegroundColor Yellow -NoNewline; Write-Host $m }

function Resumen {
    Write-Host "`n$Nombre-installer $Version`n"
    Write-Host ('  {0,-9} {1}' -f 'sistema', "windows $Arq")
    Write-Host ('  {0,-9} {1}' -f 'destino', "$Dir\$Nombre.exe")
    Write-Host ('  {0,-9} {1}' -f 'acceso', 'Menú Inicio\Logic F-4.13')
    Write-Host ''
}

function Escribible {
    try {
        $script:Dir = [IO.Path]::GetFullPath([IO.Path]::Combine((Get-Location).Path, $Dir.Trim())).TrimEnd('\')
        [void](New-Item -ItemType Directory -Force -Path $Dir)
        $prueba = Join-Path $Dir '.prueba'
        [IO.File]::WriteAllText($prueba, ''); [IO.File]::Delete($prueba)
        return $true
    } catch { return $false }
}

# Todo dentro de una funcion: con irm | iex, un exit cerraria la ventana de PowerShell del usuario
function Principal {
    $script:Arq = switch ($env:PROCESSOR_ARCHITECTURE) {
        'AMD64' { 'x86_64' }
        'ARM64' { 'aarch64' }
        default { throw "arquitectura no soportada: $env:PROCESSOR_ARCHITECTURE" }
    }
    $archivo = "$Nombre-windows-$Arq.exe"
    $url = "https://github.com/$Repo/releases/latest/download/$archivo"

    Resumen
    while (-not $Si) {
        Write-Host "  1) Instalar (por defecto)`n  2) Cambiar carpeta de destino`n  3) Cancelar"
        $op = Read-Host '>'
        if (-not $op) { $op = '1' }
        switch ($op) {
            '1' { if (Escribible) { $script:Si = $true } else { Aviso "no se puede escribir en $Dir" } }
            '2' { $nueva = Read-Host "Carpeta de destino [$Dir]"; if ($nueva) { $script:Dir = $nueva }; [void](Escribible); Resumen }
            '3' { Write-Host 'Instalación cancelada.'; return }
            default { Aviso "opción no válida: $op" }
        }
    }
    if (-not (Escribible)) { throw "no se puede escribir en $Dir" }
    Write-Host ''

    $destino = Join-Path $Dir "$Nombre.exe"
    $tmp = "$destino.tmp"
    Info "descargando $archivo desde github.com/$Repo"
    try { Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $tmp }
    catch {
        Remove-Item $tmp -ErrorAction SilentlyContinue
        throw "no se pudo descargar $url (¿hay internet? ¿ya hay un release publicado?)"
    }
    Move-Item -Force $tmp $destino
    Info "instalado en $destino"

    $nuevaTerminal = $false
    $path = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not $path) { $path = '' }
    if (($path -split ';') -contains $Dir) { Info "$Dir ya está en el PATH" }
    else {
        [Environment]::SetEnvironmentVariable('Path', ($path.TrimEnd(';') + ';' + $Dir).TrimStart(';'), 'User')
        $nuevaTerminal = $true
        Info "$Dir agregado al PATH"
    }

    $lnk = (New-Object -ComObject WScript.Shell).CreateShortcut($Acceso)
    $lnk.TargetPath = $destino
    $lnk.Save()
    Info 'acceso agregado al menú Inicio'

    Write-Host "`n  Logic_F-4:13 se instaló correctamente." -ForegroundColor Green
    Write-Host "  Ejecuta $Nombre para empezar."
    if ($nuevaTerminal) { Write-Host '  Abre una terminal nueva para que reconozca el comando.' }
    Write-Host ''
}

try { Principal }
catch { Write-Host 'error: ' -ForegroundColor Red -NoNewline; Write-Host $_.Exception.Message }
