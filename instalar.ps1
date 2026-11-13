# Instalador de Logic_F-4:13 para Windows:
#   irm https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.ps1 | iex
# Sin preguntas (instala, o actualiza si ya está instalado):
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.ps1))) -Si
param([switch]$Si, [string]$Dir)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
# La barra de Invoke-WebRequest hace la descarga mucho más lenta en Windows PowerShell 5
if ($PSVersionTable.PSVersion.Major -lt 6) { $ProgressPreference = 'SilentlyContinue' }

$Repo = 'VstevenMata/New-Logic-IDE'
$Nombre = 'Logic_F-4:13'
$Cmd = 'logicsa'
$Instalador = "https://raw.githubusercontent.com/$Repo/main/instalar.ps1"
$Acceso = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Logic F-4.13.lnk"
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

# Comando logicsa: abre la app, la actualiza o la desinstala. Las rutas quedan fijas al instalar.
$PlantillaComando = @'
# Comando logicsa: lo creó el instalador de Logic_F-4:13 y "logicsa delete" lo borra
$Nombre = 'Logic_F-4:13'
$Raiz = '__RAIZ__'
$Acceso = '__ACCESO__'
$Instalador = '__INSTALADOR__'
$Exe = Join-Path $Raiz 'app\logicsa.exe'
$Bin = Join-Path $Raiz 'bin'
$Argumentos = @($args)
$Orden = if ($Argumentos.Count) { $Argumentos[0] } else { 'open' }

function Uso {
    Write-Host 'Uso: logicsa [comando]'
    Write-Host ''
    Write-Host "  open      abre $Nombre (es lo que hace logicsa sin comando)"
    Write-Host '  update    actualiza a la última versión'
    Write-Host "  delete    desinstala $Nombre por completo; tus proyectos no se borran"
    Write-Host '  help      muestra esta ayuda'
}

if ($Orden -eq 'open') {
    if (-not (Test-Path $Exe)) { Write-Host "error: no se encontró $Exe. Reinstálalo con: logicsa update" -ForegroundColor Red; exit 1 }
    $resto = @($Argumentos | Select-Object -Skip 1)
    if ($resto.Count) { Start-Process $Exe -ArgumentList $resto } else { Start-Process $Exe }
    Write-Host "Abriendo $Nombre…"
}
elseif ($Orden -eq 'update') { & ([scriptblock]::Create((Invoke-RestMethod $Instalador))) -Si }
elseif ($Orden -in 'delete', 'uninstall') {
    if ($Argumentos[1] -ne '-y') {
        $r = Read-Host "¿Desinstalar $Nombre? Tus proyectos no se borran. [s/N]"
        if ($r -notmatch '^(s|si|sí|y)$') { Write-Host 'No se desinstaló nada.'; exit 0 }
    }
    # Si la app está abierta, su .exe no se puede borrar
    Get-Process logicsa -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $Exe } | Stop-Process -Force
    Start-Sleep -Milliseconds 300
    # Solo se borra lo que creó el instalador: la carpeta elegida pudo tener otras cosas
    Remove-Item -Force -ErrorAction SilentlyContinue $Exe, $Acceso, (Join-Path $Bin 'logicsa.cmd'), $PSCommandPath
    foreach ($d in (Join-Path $Raiz 'app'), $Bin, $Raiz) {
        if ((Test-Path $d) -and -not (Get-ChildItem -Force $d)) { Remove-Item -Force $d }
    }
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue "$env:APPDATA\LogicSa", "$env:LOCALAPPDATA\LogicSa"
    $path = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($path) { [Environment]::SetEnvironmentVariable('Path', (($path -split ';') -ne $Bin -join ';'), 'User') }
    Write-Host "$Nombre se desinstaló. Tus proyectos siguen donde los guardaste." -ForegroundColor Green
}
elseif ($Orden -in 'help', '-h', '--help') { Uso }
else { Write-Host "logicsa: comando desconocido: $Orden" -ForegroundColor Red; Uso; exit 1 }
'@

function Instalar {
    if (-not $Version) { throw 'no hay ningún release publicado todavía, o no hay conexión a internet' }
    $archivo = "$Cmd-windows-$Arq.exe"
    $app = Join-Path $Raiz 'app'
    $bin = Join-Path $Raiz 'bin'
    $exe = Join-Path $app "$Cmd.exe"
    try {
        [void](New-Item -ItemType Directory -Force -Path $app, $bin)
        $prueba = Join-Path $app '.prueba'; [IO.File]::WriteAllText($prueba, ''); [IO.File]::Delete($prueba)
    } catch { throw "no se puede escribir en $Raiz" }
    Write-Host ''

    $tmp = "$exe.tmp"
    try { Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/$Repo/releases/download/$Version/$archivo" -OutFile $tmp }
    catch { Remove-Item $tmp -ErrorAction SilentlyContinue; throw "no se pudo descargar $archivo (puede que no exista para windows $Arq)" }
    try { Move-Item -Force $tmp $exe }
    catch { Remove-Item $tmp -ErrorAction SilentlyContinue; throw "cierra $Nombre y vuelve a intentarlo" }
    Ok "Descargado $Nombre $Version (windows $Arq)"

    # logicsa.cmd es lo que se escribe en la terminal; el "& exit /b" evita un error cuando delete lo borra
    $utf8 = New-Object Text.UTF8Encoding $true
    $texto = $PlantillaComando.Replace('__RAIZ__', $Raiz.Replace("'", "''")).Replace('__ACCESO__', $Acceso.Replace("'", "''")).Replace('__INSTALADOR__', $Instalador)
    [IO.File]::WriteAllText((Join-Path $bin "$Cmd.ps1"), $texto, $utf8)
    [IO.File]::WriteAllText((Join-Path $bin "$Cmd.cmd"), "@powershell -NoProfile -ExecutionPolicy Bypass -File `"%~dp0$Cmd.ps1`" %* & exit /b`r`n")
    Ok "Comando $Cmd listo"

    $path = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not $path) { $path = '' }
    if (-not (($path -split ';') -contains $bin)) {
        [Environment]::SetEnvironmentVariable('Path', ($path.TrimEnd(';') + ';' + $bin).TrimStart(';'), 'User')
        Ok "$bin agregado al PATH"
    }
    # También en esta ventana, para poder usar logicsa sin abrir otra terminal
    if (-not (($env:Path -split ';') -contains $bin)) { $env:Path = $env:Path.TrimEnd(';') + ';' + $bin }

    $lnk = (New-Object -ComObject WScript.Shell).CreateShortcut($Acceso)
    $lnk.TargetPath = $exe
    $lnk.Save()
    Ok 'Acceso agregado al menú Inicio'

    Write-Host "`n  $Nombre se instaló correctamente.`n" -ForegroundColor Green
    Write-Host '    Versión     ' -ForegroundColor DarkGray -NoNewline; Write-Host $Version
    Write-Host '    Ubicación   ' -ForegroundColor DarkGray -NoNewline; Write-Host $Raiz
    Write-Host '    Comando     ' -ForegroundColor DarkGray -NoNewline; Write-Host $Cmd
    Write-Host "`n  Para empezar:"
    Write-Host ('    {0,-16}' -f $Cmd) -ForegroundColor Cyan -NoNewline; Write-Host ' abre la app'
    Write-Host ('    {0,-16}' -f "$Cmd update") -ForegroundColor Cyan -NoNewline; Write-Host ' actualiza a la última versión'
    Write-Host ('    {0,-16}' -f "$Cmd delete") -ForegroundColor Cyan -NoNewline; Write-Host ' la desinstala (tus proyectos no se borran)'
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

    # Si ya está instalado (su carpeta bin está en el PATH del usuario), se actualiza en la misma carpeta
    $previa = ([Environment]::GetEnvironmentVariable('Path', 'User') -split ';') |
        Where-Object { $_ -and (Test-Path (Join-Path $_ "$Cmd.ps1")) } | Select-Object -First 1
    $script:Raiz = if ($Dir) { Join-Path ([IO.Path]::GetFullPath($Dir)) 'LogicSa' }
                   elseif ($previa) { Split-Path $previa }
                   else { "$env:LOCALAPPDATA\Programs\LogicSa" }

    # Versión del último release (sin release o sin internet queda vacía)
    $script:Version = ''
    try {
        $peticion = [Net.WebRequest]::Create("https://github.com/$Repo/releases/latest")
        $peticion.Method = 'HEAD'
        $respuesta = $peticion.GetResponse()
        if ($respuesta.ResponseUri.AbsoluteUri -match '/tag/([^/]+)$') { $script:Version = $Matches[1] }
        $respuesta.Close()
    } catch {}

    Write-Host "`n  ✻ $Nombre" -ForegroundColor Cyan
    $linea = "  Instalador · windows $Arq"
    if ($Version) { $linea += " · $Version" }
    Write-Host $linea -ForegroundColor DarkGray

    $elegida = 0
    if ($previa) {
        if (-not $Si) { $elegida = Menu "$Nombre ya está instalado. ¿Qué quieres hacer?" @('Actualizar a la última versión', 'Desinstalar', 'Cancelar') }
        switch ($elegida) {
            0 { Instalar }
            1 { Write-Host ''; & (Join-Path $previa "$Cmd.ps1") delete -y; Write-Host '' }
            default { Cancelar }
        }
    } else {
        if (-not $Si) { $elegida = Menu '¿Cómo quieres instalarlo?' @("Instalar en $Raiz (recomendado)", 'Elegir otra carpeta', 'Cancelar') }
        switch ($elegida) {
            0 { Instalar }
            1 {
                $nueva = Read-Host "`n  Carpeta donde instalar (se crea LogicSa\ adentro)"
                if ($nueva) { $script:Raiz = Join-Path ([IO.Path]::GetFullPath([IO.Path]::Combine((Get-Location).Path, $nueva))) 'LogicSa' }
                Instalar
            }
            default { Cancelar }
        }
    }
}

try { Principal }
catch { Write-Host "  ✖ $($_.Exception.Message)" -ForegroundColor Red }
