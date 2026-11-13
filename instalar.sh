#!/usr/bin/env bash
# Instalador de Logic_F-4:13 para Linux y macOS:
#   curl -fsSL https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.sh | bash
# Sin preguntas (instala, o actualiza si ya está instalado):
#   curl -fsSL https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.sh | bash -s -- -y
set -euo pipefail

REPO=VstevenMata/New-Logic-IDE
NOMBRE=Logic_F-4:13
CMD=logicsa
INSTALADOR=https://raw.githubusercontent.com/$REPO/main/instalar.sh

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
    B=$'\e[1m' T=$'\e[2m' R=$'\e[0m' ACENTO=$'\e[1;36m' VERDE=$'\e[32m' AMARILLO=$'\e[33m' ROJO=$'\e[31m'
else
    B='' T='' R='' ACENTO='' VERDE='' AMARILLO='' ROJO=''
fi
ok() { printf '  %s✔%s %s\n' "$VERDE" "$R" "$*"; }
aviso() { printf '  %s!%s %s\n' "$AMARILLO$B" "$R" "$*"; }
error() { printf '  %s✖ %s%s\n' "$ROJO$B" "$*" "$R" >&2; exit 1; }
corto() { printf '%s' "${1/#$HOME/\~}"; }

uso() {
    cat <<EOF
Instalador de $NOMBRE

uso: instalar.sh [opciones]

  -y, --yes        instala sin preguntar (si ya está instalado, lo actualiza)
      --dir RUTA   carpeta donde instalar (se crea $CMD/ adentro)
  -h, --help       muestra esta ayuda
EOF
}

# Carpeta elegida → APP=<carpeta>/logicsa (absoluta, sin comillas que rompan el comando generado)
fijar_dir() {
    local d=${1/#\~/$HOME}
    case $d in /*) ;; *) d="$PWD/$d" ;; esac
    case $d in *\'*) error "la ruta no puede tener comillas simples" ;; esac
    APP="${d%/}/$CMD"
}

SI=0 DIR_DADO=0
APP="$HOME/.local/share/$CMD"
while [ $# -gt 0 ]; do
    case $1 in
        -y | --yes) SI=1 ;;
        --dir) fijar_dir "${2:?falta la ruta después de --dir}"; DIR_DADO=1; shift ;;
        -h | --help) uso; exit 0 ;;
        *) error "opción desconocida: $1 (usa --help)" ;;
    esac
    shift
done

case "$(uname -s)" in
    Linux) SO=linux ;;
    Darwin) SO=macos ;;
    *) error "sistema no soportado: $(uname -s). En Windows usa instalar.ps1" ;;
esac
case "$(uname -m)" in
    x86_64 | amd64) ARQ=x86_64 ;;
    arm64 | aarch64) ARQ=aarch64 ;;
    *) error "arquitectura no soportada: $(uname -m)" ;;
esac
command -v curl >/dev/null || error "se necesita curl"

BIN="$HOME/.local/bin"
COMANDO="$BIN/$CMD"
LINEA_PATH="export PATH=\"\$HOME/.local/bin:\$PATH\" # $CMD"
FISH="$HOME/.config/fish/conf.d/$CMD.fish"
# Carpetas de datos de la app: "logicsa delete" las borra. Los proyectos de los estudiantes NO deben guardarse aquí.
if [ $SO = linux ]; then
    CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/$CMD"
    CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/$CMD"
    DATOS="${XDG_DATA_HOME:-$HOME/.local/share}/$CMD"
    DESKTOP="${XDG_DATA_HOME:-$HOME/.local/share}/applications/$CMD.desktop"
else
    CONFIG="$HOME/Library/Application Support/$CMD"
    CACHE="$HOME/Library/Caches/$CMD"
    DATOS="$HOME/.local/share/$CMD"
    DESKTOP=""
fi

# Si ya está instalado, se actualiza en la misma carpeta
INSTALADO=0
if [ -f "$COMANDO" ]; then
    INSTALADO=1
    previa=$(sed -n "s/^APP='\(.*\)'\$/\1/p" "$COMANDO")
    if [ $DIR_DADO = 0 ] && [ -n "$previa" ]; then APP=$previa; fi
fi

# Versión del último release (sin release o sin internet queda vacía)
url=$(curl -fsSLI -o /dev/null -w '%{url_effective}' --connect-timeout 10 "https://github.com/$REPO/releases/latest" 2>/dev/null || true)
case $url in */tag/*) VERSION=${url##*/} ;; *) VERSION="" ;; esac

# Con curl | bash la entrada es el script: las teclas se leen de la terminal. Sin terminal, se instala sin preguntar.
INTERACTIVO=0 ESTADO="" TMP=""
if [ $SI = 0 ] && [ -t 1 ] && { exec 3</dev/tty; } 2>/dev/null; then INTERACTIVO=1; fi
salir() {
    if [ -n "$TMP" ]; then rm -f "$TMP"; fi
    if [ -n "$ESTADO" ]; then stty "$ESTADO" <&3 2>/dev/null || true; fi
    if [ $INTERACTIVO = 1 ]; then printf '\e[?25h'; fi
}
trap salir EXIT
trap 'echo; exit 130' INT
# bash 3.2 (el de macOS) no acepta tiempos con decimales en read -t
ESPERA=0.05
if [ "${BASH_VERSINFO[0]}" -lt 4 ]; then ESPERA=1; fi

# menu "pregunta" "opción" …: se maneja con ↑↓ (o j/k, o el número) y deja en ELEGIDA 0, 1, … o -1 si se sale
menu() {
    local pregunta=$1 sel=0 fin="" n i op tecla resto
    shift; n=$#
    printf '\n  %s%s%s\n\n' "$B" "$pregunta" "$R"
    ESTADO=$(stty -g <&3); stty -echo -icanon <&3
    printf '\e[?25l'
    while :; do
        i=0
        for op in "$@"; do
            if [ $i = "$sel" ]; then printf '\e[2K  %s❯ %d. %s%s\n' "$ACENTO" $((i + 1)) "$op" "$R"
            else printf '\e[2K    %d. %s\n' $((i + 1)) "$op"; fi
            i=$((i + 1))
        done
        printf '\n\e[2K  %s↑↓ para moverte · Enter para elegir · Esc para salir%s\n' "$T" "$R"
        if [ "$fin" ]; then break; fi
        IFS= read -rsn1 tecla <&3 || tecla=q
        case $tecla in
            $'\e')
                resto=""
                IFS= read -rsn2 -t $ESPERA resto <&3 || true
                case $resto in
                    '[A' | OA) sel=$(((sel + n - 1) % n)) ;;
                    '[B' | OB) sel=$(((sel + 1) % n)) ;;
                    '') sel=-1 fin=1 ;;
                esac
                ;;
            k) sel=$(((sel + n - 1) % n)) ;;
            j) sel=$(((sel + 1) % n)) ;;
            [1-9]) if [ "$tecla" -le $n ]; then sel=$((tecla - 1)) fin=1; fi ;;
            '') fin=1 ;;
            q) sel=-1 fin=1 ;;
        esac
        printf '\e[%dA' $((n + 2))
    done
    printf '\e[?25h'
    stty "$ESTADO" <&3; ESTADO=""
    ELEGIDA=$sel
}

pedir_carpeta() {
    local nueva
    echo
    read -e -r -p "  Carpeta donde instalar (se crea $CMD/ adentro): " nueva </dev/tty || nueva=""
    if [ -n "$nueva" ]; then fijar_dir "$nueva"; fi
}

# Comando logicsa: abre la app, la actualiza o la desinstala. Las rutas quedan fijas al instalar.
escribir_comando() {
    {
        printf '#!/bin/sh\n# Comando %s: lo creó el instalador de %s y "%s delete" lo borra\n' "$CMD" "$NOMBRE" "$CMD"
        printf "NOMBRE='%s'\nAPP='%s'\nCOMANDO='%s'\nDESKTOP='%s'\nFISH='%s'\n" "$NOMBRE" "$APP" "$COMANDO" "$DESKTOP" "$FISH"
        printf "CONFIG='%s'\nCACHE='%s'\nDATOS='%s'\n" "$CONFIG" "$CACHE" "$DATOS"
        printf "LINEA_PATH='%s'\nINSTALADOR='%s'\n" "$LINEA_PATH" "$INSTALADOR"
        cat <<'EOF'

uso() {
    cat <<FIN
Uso: logicsa [comando]

  open      abre $NOMBRE (es lo que hace logicsa sin comando)
  update    actualiza a la última versión
  delete    desinstala $NOMBRE por completo; tus proyectos no se borran
  help      muestra esta ayuda
FIN
}

case "${1:-open}" in
    open)
        if [ $# -gt 0 ]; then shift; fi
        if [ ! -x "$APP/logicsa" ]; then
            echo "error: no se encontró $APP/logicsa. Reinstálalo con: logicsa update" >&2
            exit 1
        fi
        nohup "$APP/logicsa" "$@" >/dev/null 2>&1 &
        echo "Abriendo $NOMBRE…"
        ;;
    update)
        curl -fsSL "$INSTALADOR" | bash -s -- -y
        ;;
    delete | uninstall)
        if [ "${2:-}" != -y ]; then
            printf '¿Desinstalar %s? Tus proyectos no se borran. [s/N] ' "$NOMBRE"
            read -r r || r=""
            case $r in s | S | si | Si | sí | Sí | y | Y) ;; *) echo "No se desinstaló nada."; exit 0 ;; esac
        fi
        # Solo se borra lo que creó el instalador: la carpeta elegida pudo tener otras cosas
        rm -f "$APP/logicsa"
        rmdir "$APP" 2>/dev/null
        rm -rf "$CONFIG" "$CACHE" "$DATOS"
        rm -f "$DESKTOP" "$FISH"
        for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
            if [ -f "$rc" ] && grep -qxF "$LINEA_PATH" "$rc"; then
                grep -vxF "$LINEA_PATH" "$rc" >"$rc.logicsa"
                cat "$rc.logicsa" >"$rc"
                rm -f "$rc.logicsa"
            fi
        done
        rm -f "$COMANDO"
        echo "✔ $NOMBRE se desinstaló. Tus proyectos siguen donde los guardaste."
        ;;
    help | -h | --help) uso ;;
    *)
        echo "logicsa: comando desconocido: $1" >&2
        uso >&2
        exit 1
        ;;
esac
EOF
    } >"$COMANDO.tmp"
    chmod +x "$COMANDO.tmp"
    mv -f "$COMANDO.tmp" "$COMANDO"
}

# ~/.local/bin ya está en el PATH en la mayoría de sistemas; si no, se agrega para bash, zsh y fish
agregar_path() {
    local rc
    case ":$PATH:" in *":$BIN:"*) return 0 ;; esac
    NUEVA_TERMINAL=1
    case ${SHELL##*/} in bash) touch "$HOME/.bashrc" ;; zsh) touch "$HOME/.zshrc" ;; esac
    for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
        if [ -f "$rc" ] && ! grep -qxF "$LINEA_PATH" "$rc"; then
            if [ -s "$rc" ] && [ -n "$(tail -c1 "$rc")" ]; then echo >>"$rc"; fi
            echo "$LINEA_PATH" >>"$rc"
        fi
    done
    if command -v fish >/dev/null; then
        mkdir -p "${FISH%/*}"
        echo 'fish_add_path ~/.local/bin' >"$FISH"
    fi
    ok "~/.local/bin agregado al PATH"
}

instalar() {
    local archivo="$CMD-$SO-$ARQ"
    echo
    [ -n "$VERSION" ] || error "no hay ningún release publicado todavía, o no hay conexión a internet"
    if ! mkdir -p "$APP" "$BIN" 2>/dev/null || [ ! -w "$APP" ]; then error "no se puede escribir en $(corto "$APP")"; fi
    TMP="$APP/$CMD.tmp"
    if [ -t 2 ]; then
        curl -fL --progress-bar --connect-timeout 10 -o "$TMP" "https://github.com/$REPO/releases/download/$VERSION/$archivo" ||
            error "no se pudo descargar $archivo (puede que no exista para $SO $ARQ)"
        printf '\e[1A\e[2K' >&2
    else
        curl -fsSL --connect-timeout 10 -o "$TMP" "https://github.com/$REPO/releases/download/$VERSION/$archivo" ||
            error "no se pudo descargar $archivo (puede que no exista para $SO $ARQ)"
    fi
    chmod +x "$TMP"
    mv -f "$TMP" "$APP/$CMD"
    TMP=""
    ok "Descargado $NOMBRE $VERSION ($SO $ARQ)"

    escribir_comando
    ok "Comando $CMD listo"
    NUEVA_TERMINAL=0
    agregar_path

    if [ -n "$DESKTOP" ]; then
        mkdir -p "${DESKTOP%/*}"
        cat >"$DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Name=$NOMBRE
Comment=Diseña, visualiza y simula circuitos digitales
Exec="$APP/$CMD"
Terminal=false
Categories=Development;Education;Electronics;
EOF
        ok "Acceso agregado al menú de aplicaciones"
    fi

    printf '\n  %s%s se instaló correctamente.%s\n\n' "$VERDE$B" "$NOMBRE" "$R"
    printf '    %sVersión%s     %s\n' "$T" "$R" "$VERSION"
    printf '    %sUbicación%s   %s\n' "$T" "$R" "$(corto "$APP")"
    printf '    %sComando%s     %s\n\n' "$T" "$R" "$(corto "$COMANDO")"
    printf '  Para empezar:\n'
    printf '    %s%-16s%s abre la app\n' "$ACENTO" "$CMD" "$R"
    printf '    %s%-16s%s actualiza a la última versión\n' "$ACENTO" "$CMD update" "$R"
    printf '    %s%-16s%s la desinstala (tus proyectos no se borran)\n' "$ACENTO" "$CMD delete" "$R"
    if [ $NUEVA_TERMINAL = 1 ]; then printf '\n'; aviso "Abre una terminal nueva para que reconozca el comando $CMD."; fi
    echo
}

cancelar() { printf '\n  No se hizo ningún cambio.\n\n'; }

printf '\n  %s✻ %s%s\n' "$ACENTO" "$NOMBRE" "$R"
printf '  %sInstalador · %s %s%s%s\n' "$T" "$SO" "$ARQ" "${VERSION:+ · $VERSION}" "$R"

if [ $INSTALADO = 1 ]; then
    ELEGIDA=0
    if [ $INTERACTIVO = 1 ]; then
        menu "$NOMBRE ya está instalado. ¿Qué quieres hacer?" "Actualizar a la última versión" "Desinstalar" "Cancelar"
    fi
    case $ELEGIDA in
        0) instalar ;;
        1) echo; sh "$COMANDO" delete -y; echo ;;
        *) cancelar ;;
    esac
else
    if [ -z "$VERSION" ]; then echo; error "no hay ningún release publicado todavía, o no hay conexión a internet"; fi
    ELEGIDA=0
    if [ $INTERACTIVO = 1 ]; then
        menu "¿Cómo quieres instalarlo?" "Instalar en $(corto "$APP") (recomendado)" "Elegir otra carpeta" "Cancelar"
    fi
    case $ELEGIDA in
        0) instalar ;;
        1) pedir_carpeta; instalar ;;
        *) cancelar ;;
    esac
fi
