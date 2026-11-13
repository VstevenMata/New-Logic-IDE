# Instalador de LogicSM para Windows:
#   irm https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.ps1 | iex
# Sin preguntas (instala, o actualiza si ya está instalado):
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.ps1))) -Si
# LogicSM lleva este script adentro: "logicsm update" y "logicsm delete" lo abren en una ventana nueva.
param([switch]$Si, [string]$Dir, [switch]$Desinstalar, [int]$Esperar)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
# La barra de Invoke-WebRequest hace la descarga mucho más lenta en Windows PowerShell 5
if ($PSVersionTable.PSVersion.Major -lt 6) { $ProgressPreference = 'SilentlyContinue' }

$Repo = 'VstevenMata/New-Logic-IDE'
$Nombre = 'LogicSM'
$Cmd = 'logicsm'
$Acceso = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\LogicSM.lnk"
# Carpetas de datos de la app: la desinstalación las borra. Los proyectos de los estudiantes NO deben guardarse aquí.
$Datos = @("$env:APPDATA\LogicSM", "$env:LOCALAPPDATA\LogicSM")
# La consola clásica no tiene todos los símbolos; Windows Terminal y PowerShell 7 sí
$Unicode = $env:WT_SESSION -or $PSVersionTable.PSVersion.Major -ge 7
$Ok = if ($Unicode) { '✔' } else { '√' }
$Flecha = if ($Unicode) { '❯' } else { '>' }

function Ok($m) { Write-Host "  $Ok " -ForegroundColor Green -NoNewline; Write-Host $m }

# menu: se maneja con ↑↓ (o el número) y devuelve 0, 1, … o -1 si se sale con Esc
function Menu($pregunta, [string[]]$opciones) {
    Write-Host "`n  $pregunta`n" -ForegroundColor White
    $sel = 0; $n = $opciones.Count; $fin = $false
    [Console]::CursorVisible = $false
    try {
        while ($true) {
            $ancho = [Console]::WindowWidth - 1
            for ($i = 0; $i -lt $n; $i++) {
                if ($i -eq $sel) { Write-Host ("  $Flecha $($i + 1). $($opciones[$i])").PadRight($ancho) -ForegroundColor Cyan }
                else { Write-Host ("    $($i + 1). $($opciones[$i])").PadRight($ancho) }
            }
            Write-Host ''
            Write-Host '  ↑↓ para moverte · Enter para elegir · Esc para salir' -ForegroundColor DarkGray
            if ($fin) { return $sel }
            # Se calcula después de dibujar porque la consola pudo desplazarse
            $arriba = [Console]::CursorTop - ($n + 2)
            $k = [Console]::ReadKey($true)
            if ($k.Key -eq 'UpArrow') { $sel = ($sel + $n - 1) % $n }
            elseif ($k.Key -eq 'DownArrow') { $sel = ($sel + 1) % $n }
            elseif ($k.Key -eq 'Enter') { $fin = $true }
            elseif ($k.Key -eq 'Escape') { $sel = -1; $fin = $true }
            elseif ($k.KeyChar -match '^[1-9]$' -and [int][string]$k.KeyChar -le $n) { $sel = [int][string]$k.KeyChar - 1; $fin = $true }
            [Console]::SetCursorPosition(0, $arriba)
        }
    } finally { [Console]::CursorVisible = $true }
}

function QuitarDelPath($carpeta) {
    $path = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($path) { [Environment]::SetEnvironmentVariable('Path', (($path -split ';') -ne $carpeta -join ';'), 'User') }
}

function Instalar {
    if (-not $Version) { throw 'no hay ningún release publicado todavía, o no hay conexión a internet' }
    $archivo = "$Cmd-windows-$Arq.exe"
    $exe = Join-Path $Raiz "$Cmd.exe"
    try {
        [void](New-Item -ItemType Directory -Force -Path $Raiz)
        $prueba = Join-Path $Raiz '.prueba'; [IO.File]::WriteAllText($prueba, ''); [IO.File]::Delete($prueba)
    } catch { throw "no se puede escribir en $Raiz" }
    Write-Host ''

    $tmp = "$exe.tmp"
    try { Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/$Repo/releases/download/$Version/$archivo" -OutFile $tmp }
    catch { Remove-Item $tmp -ErrorAction SilentlyContinue; throw "no se pudo descargar $archivo (puede que no exista para windows $Arq)" }
    try { Move-Item -Force $tmp $exe }
    catch { Remove-Item $tmp -ErrorAction SilentlyContinue; throw "cierra $Nombre y vuelve a intentarlo" }
    Ok "Descargado $Nombre $Version (windows $Arq)"

    # Si antes estaba en otra carpeta, se quita la copia vieja
    if ($Previa -and $Previa -ne $Raiz) {
        Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $Previa "$Cmd.exe")
        QuitarDelPath $Previa
    }
    $path = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not $path) { $path = '' }
    if (-not (($path -split ';') -contains $Raiz)) {
        [Environment]::SetEnvironmentVariable('Path', ($path.TrimEnd(';') + ';' + $Raiz).TrimStart(';'), 'User')
        Ok "$Raiz agregado al PATH"
    }
    # También en esta ventana, para poder usar logicsm sin abrir otra terminal
    if (-not (($env:Path -split ';') -contains $Raiz)) { $env:Path = $env:Path.TrimEnd(';') + ';' + $Raiz }

    # El .exe ya trae el logo como ícono
    $lnk = (New-Object -ComObject WScript.Shell).CreateShortcut($Acceso)
    $lnk.TargetPath = $exe
    $lnk.Save()
    Ok 'Acceso agregado al menú Inicio'

    Write-Host "`n  $Nombre se instaló correctamente.`n" -ForegroundColor Green
    Write-Host '    Versión     ' -ForegroundColor DarkGray -NoNewline; Write-Host $Version
    Write-Host '    Ubicación   ' -ForegroundColor DarkGray -NoNewline; Write-Host $exe
    Write-Host "`n  Para empezar:"
    Write-Host ('    {0,-16}' -f $Cmd) -ForegroundColor Cyan -NoNewline; Write-Host ' abre la app'
    Write-Host ('    {0,-16}' -f "$Cmd update") -ForegroundColor Cyan -NoNewline; Write-Host ' actualiza a la última versión'
    Write-Host ('    {0,-16}' -f "$Cmd delete") -ForegroundColor Cyan -NoNewline; Write-Host ' la desinstala (tus proyectos no se borran)'
    Write-Host ''
}

# Solo se borra lo que creó el instalador: si el .exe estaba en otra carpeta, esa carpeta se queda
function Quitar {
    $exe = Join-Path $Previa "$Cmd.exe"
    # Si la app está abierta, su .exe no se puede borrar
    Get-Process $Cmd -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe } | Stop-Process -Force
    Start-Sleep -Milliseconds 300
    Remove-Item -Force -ErrorAction SilentlyContinue $exe, $Acceso
    if ((Test-Path $Previa) -and -not (Get-ChildItem -Force $Previa)) { Remove-Item -Force $Previa }
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $Datos
    QuitarDelPath $Previa
    Write-Host ''
    Ok "$Nombre se desinstaló. Tus proyectos siguen donde los guardaste."
    Write-Host ''
}

function Cancelar { Write-Host "`n  No se hizo ningún cambio.`n" }

# Todo dentro de una función: con irm | iex, un exit cerraría la ventana de PowerShell del usuario
function Principal {
    $script:Arq = switch ($env:PROCESSOR_ARCHITECTURE) {
        'AMD64' { 'x86_64' }
        'ARM64' { 'aarch64' }
        default { throw "arquitectura no soportada: $env:PROCESSOR_ARCHITECTURE" }
    }

    # logicsm update/delete abren este instalador y se cierran: se espera a que suelten el .exe
    if ($Esperar) { Wait-Process -Id $Esperar -Timeout 30 -ErrorAction SilentlyContinue }

    # Instalación existente: la carpeta del PATH del usuario que tiene logicsm.exe
    $script:Previa = ([Environment]::GetEnvironmentVariable('Path', 'User') -split ';') |
        Where-Object { $_ -and (Test-Path (Join-Path $_ "$Cmd.exe")) } | Select-Object -First 1
    $script:Raiz = if ($Dir) { [IO.Path]::GetFullPath($Dir).TrimEnd('\') }
                   elseif ($Previa) { $Previa }
                   else { "$env:LOCALAPPDATA\Programs\LogicSM" }

    # Versión del último release (sin release o sin internet queda vacía); desinstalar no la necesita
    $script:Version = ''
    if (-not $Desinstalar) {
        try {
            $peticion = [Net.WebRequest]::Create("https://github.com/$Repo/releases/latest")
            $peticion.Method = 'HEAD'
            $respuesta = $peticion.GetResponse()
            if ($respuesta.ResponseUri.AbsoluteUri -match '/tag/([^/]+)$') { $script:Version = $Matches[1] }
            $respuesta.Close()
        } catch {}
    }

    Write-Host "`n  ✻ $Nombre" -ForegroundColor Cyan
    $linea = "  Instalador · windows $Arq"
    if ($Version) { $linea += " · $Version" }
    Write-Host $linea -ForegroundColor DarkGray

    if ($Desinstalar) {
        if (-not $Previa) { Write-Host "`n  $Nombre no está instalado.`n"; return }
        $elegida = 1
        if (-not $Si) { $elegida = Menu "¿Desinstalar $Nombre? Tus proyectos no se borran." @('Cancelar', 'Desinstalar') }
        if ($elegida -eq 1) { Quitar } else { Cancelar }
    } elseif ($Previa) {
        $elegida = 0
        if (-not $Si) { $elegida = Menu "$Nombre ya está instalado. ¿Qué quieres hacer?" @('Actualizar a la última versión', 'Desinstalar', 'Cancelar') }
        switch ($elegida) { 0 { Instalar } 1 { Quitar } default { Cancelar } }
    } else {
        if (-not $Version) { throw 'no hay ningún release publicado todavía, o no hay conexión a internet' }
        $elegida = 0
        if (-not $Si) { $elegida = Menu '¿Cómo quieres instalarlo?' @("Instalar en $Raiz (recomendado)", 'Elegir otra carpeta', 'Cancelar') }
        switch ($elegida) {
            0 { Instalar }
            1 {
                $nueva = Read-Host "`n  Carpeta donde guardar $Nombre (si no existe se crea)"
                if ($nueva) { $script:Raiz = [IO.Path]::GetFullPath([IO.Path]::Combine((Get-Location).Path, $nueva)).TrimEnd('\') }
                Instalar
            }
            default { Cancelar }
        }
    }
}

try { Principal }
catch { Write-Host "  ✖ $($_.Exception.Message)" -ForegroundColor Red }
# Abierto por logicsm update/delete en una ventana propia: que no se cierre antes de leerlo
if ($Esperar) { [void](Read-Host "`n  Presiona Enter para cerrar") }
