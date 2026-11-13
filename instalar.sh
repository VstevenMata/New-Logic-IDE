#!/usr/bin/env bash
# Instalador de LogicSM para Linux y macOS:
#   curl -fsSL https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.sh | bash
# Sin preguntas (instala, o actualiza si ya está instalado):
#   curl -fsSL https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.sh | bash -s -- -y
# LogicSM lleva este script adentro: "logicsm update" y "logicsm delete" lo usan.
set -euo pipefail

REPO=VstevenMata/New-Logic-IDE
NOMBRE=LogicSM
CMD=logicsm

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

  -y, --yes        sin preguntas (instala, o actualiza si ya está instalado)
      --dir RUTA   carpeta donde guardar el ejecutable (por defecto ~/.local/bin)
      --desinstalar  quita $NOMBRE del sistema; los proyectos no se borran
  -h, --help       muestra esta ayuda
EOF
}

# ~ y rutas relativas a absolutas, sin / final
fijar_dir() {
    local d=${1/#\~/$HOME}
    case $d in /*) ;; *) d="$PWD/$d" ;; esac
    DIR=${d%/}
}

BIN="$HOME/.local/bin"
SI=0 DESINSTALAR=0 DIR_DADO=0 DIR=$BIN
while [ $# -gt 0 ]; do
    case $1 in
        -y | --yes) SI=1 ;;
        --dir) fijar_dir "${2:?falta la ruta después de --dir}"; DIR_DADO=1; shift ;;
        --desinstalar) DESINSTALAR=1 ;;
        -h | --help) uso; exit 0 ;;
        *) error "opción desconocida: $1 (usa --help)" ;;
    esac
    shift
done

# NixOS no puede correr el ejecutable suelto (no tiene /lib64/ld-linux); el flake lo compila y lo envuelve
if [ -e /etc/NIXOS ]; then
    printf '\n  En NixOS, LogicSM se abre con nix:\n\n    %snix run github:%s%s\n\n' "$ACENTO" "$REPO" "$R"
    exit 0
fi

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

ENLACE="$BIN/$CMD"
LINEA_PATH="export PATH=\"\$HOME/.local/bin:\$PATH\" # $CMD"
FISH="$HOME/.config/fish/conf.d/$CMD.fish"
# Carpetas de datos de la app: la desinstalación las borra. Los proyectos de los estudiantes NO deben guardarse aquí.
if [ $SO = linux ]; then
    CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/$CMD"
    CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/$CMD"
    DATOS="${XDG_DATA_HOME:-$HOME/.local/share}/$CMD"
    DESKTOP="${XDG_DATA_HOME:-$HOME/.local/share}/applications/$CMD.desktop"
    ICONO="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/512x512/apps/$CMD.png"
else
    CONFIG="$HOME/Library/Application Support/$CMD"
    CACHE="$HOME/Library/Caches/$CMD"
    DATOS="" DESKTOP="" ICONO=""
fi

# Instalación existente: ~/.local/bin/logicsm es el ejecutable, o un enlace a él si se eligió otra carpeta
ACTUAL=""
if [ -L "$ENLACE" ]; then ACTUAL=$(readlink "$ENLACE"); elif [ -f "$ENLACE" ]; then ACTUAL=$ENLACE; fi
if [ -n "$ACTUAL" ] && [ $DIR_DADO = 0 ]; then DIR=${ACTUAL%/*}; fi
# Versión instalada ("LogicSM 0.1.1" → v0.1.1), para decir "se actualizó de … a …"
INSTALADA=""
if [ -n "$ACTUAL" ]; then
    INSTALADA=$("$ACTUAL" --version 2>/dev/null | awk '{print $2}' || true)
    INSTALADA=${INSTALADA:+v$INSTALADA}
fi

# Versión del último release (sin release o sin internet queda vacía); desinstalar no la necesita
VERSION=""
if [ $DESINSTALAR = 0 ]; then
    command -v curl >/dev/null || error "se necesita curl"
    url=$(curl -fsSLI -o /dev/null -w '%{url_effective}' --connect-timeout 10 "https://github.com/$REPO/releases/latest" 2>/dev/null || true)
    case $url in */tag/*) VERSION=${url##*/} ;; esac
fi

# Con curl | bash la entrada es el script: las teclas se leen de la terminal. Sin terminal, no se pregunta.
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
    read -e -r -p "  Carpeta donde guardar $NOMBRE (si no existe se crea): " nueva </dev/tty || nueva=""
    if [ -n "$nueva" ]; then fijar_dir "$nueva"; fi
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
    local archivo="$CMD-$SO-$ARQ" destino="$DIR/$CMD"
    echo
    [ -n "$VERSION" ] || error "no hay ningún release publicado todavía, o no hay conexión a internet"
    if [ "$INSTALADA" = "$VERSION" ] && [ "$destino" = "$ACTUAL" ]; then
        ok "Ya tienes la última versión ($VERSION)."
        echo
        return
    fi
    if ! mkdir -p "$DIR" "$BIN" 2>/dev/null || [ ! -w "$DIR" ]; then error "no se puede escribir en $(corto "$DIR")"; fi
    TMP="$destino.tmp"
    if [ -t 2 ]; then
        curl -fL --progress-bar --connect-timeout 10 -o "$TMP" "https://github.com/$REPO/releases/download/$VERSION/$archivo" ||
            error "no se pudo descargar $archivo (puede que no exista para $SO $ARQ)"
        printf '\e[1A\e[2K' >&2
    else
        curl -fsSL --connect-timeout 10 -o "$TMP" "https://github.com/$REPO/releases/download/$VERSION/$archivo" ||
            error "no se pudo descargar $archivo (puede que no exista para $SO $ARQ)"
    fi
    chmod +x "$TMP"
    mv -f "$TMP" "$destino"
    TMP=""
    ok "Descargado $NOMBRE $VERSION ($SO $ARQ)"

    # Si antes estaba en otra carpeta, se quita la copia vieja
    if [ -n "$ACTUAL" ] && [ "$ACTUAL" != "$destino" ]; then rm -f "$ACTUAL"; fi
    # En otra carpeta, el comando logicsm apunta ahí desde ~/.local/bin (como en GGEZ)
    if [ "$destino" != "$ENLACE" ]; then
        ln -sf "$destino" "$ENLACE"
        ok "Comando $CMD → $(corto "$destino")"
    fi
    NUEVA_TERMINAL=0
    agregar_path

    if [ -n "$DESKTOP" ]; then
        # El ícono es opcional: si no se puede bajar, el acceso queda con el ícono genérico
        mkdir -p "${ICONO%/*}" "${DESKTOP%/*}"
        curl -fsL --connect-timeout 10 -o "$ICONO" "https://raw.githubusercontent.com/$REPO/$VERSION/assets/logo.png" || true
        cat >"$DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Name=$NOMBRE
Comment=Diseña, visualiza y simula circuitos digitales
Exec="$destino"
Icon=$ICONO
StartupWMClass=$CMD
Terminal=false
Categories=Development;Education;Electronics;
EOF
        ok "Acceso agregado al menú de aplicaciones"
    fi

    if [ -n "$ACTUAL" ]; then
        printf '\n  %s%s se actualizó correctamente.%s\n\n' "$VERDE$B" "$NOMBRE" "$R"
        printf '    %sVersión%s     %s\n' "$T" "$R" "${INSTALADA:+$INSTALADA → }$VERSION"
        printf '    %sUbicación%s   %s\n\n' "$T" "$R" "$(corto "$destino")"
        return
    fi
    printf '\n  %s%s se instaló correctamente.%s\n\n' "$VERDE$B" "$NOMBRE" "$R"
    printf '    %sVersión%s     %s\n' "$T" "$R" "$VERSION"
    printf '    %sUbicación%s   %s\n\n' "$T" "$R" "$(corto "$destino")"
    printf '  Para empezar:\n'
    printf '    %s%-16s%s abre la app\n' "$ACENTO" "$CMD" "$R"
    printf '    %s%-16s%s actualiza a la última versión\n' "$ACENTO" "$CMD update" "$R"
    printf '    %s%-16s%s la desinstala (tus proyectos no se borran)\n' "$ACENTO" "$CMD delete" "$R"
    if [ $NUEVA_TERMINAL = 1 ]; then printf '\n'; aviso "Abre una terminal nueva para que reconozca el comando $CMD."; fi
    echo
}

# Solo se borra lo que creó el instalador: si el ejecutable estaba en otra carpeta, esa carpeta se queda
desinstalar() {
    local rc
    echo
    rm -f "$ACTUAL" "$ENLACE" "$DESKTOP" "$ICONO" "$FISH"
    rm -rf "$CONFIG" "$CACHE" "$DATOS"
    for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
        if [ -f "$rc" ] && grep -qxF "$LINEA_PATH" "$rc"; then
            grep -vxF "$LINEA_PATH" "$rc" >"$rc.$CMD" || true
            cat "$rc.$CMD" >"$rc"
            rm -f "$rc.$CMD"
        fi
    done
    ok "$NOMBRE se desinstaló. Tus proyectos siguen donde los guardaste."
    echo
}

cancelar() { printf '\n  No se hizo ningún cambio.\n\n'; }

printf '\n  %s✻ %s%s\n' "$ACENTO" "$NOMBRE" "$R"
printf '  %sInstalador · %s %s%s%s\n' "$T" "$SO" "$ARQ" "${VERSION:+ · $VERSION}" "$R"

if [ $DESINSTALAR = 1 ]; then
    if [ -z "$ACTUAL" ]; then printf '\n'; aviso "$NOMBRE no está instalado."; echo; exit 0; fi
    ELEGIDA=1
    if [ $INTERACTIVO = 1 ]; then menu "¿Desinstalar $NOMBRE? Tus proyectos no se borran." "Cancelar" "Desinstalar"; fi
    case $ELEGIDA in 1) desinstalar ;; *) cancelar ;; esac
elif [ -n "$ACTUAL" ]; then
    ELEGIDA=0
    if [ $INTERACTIVO = 1 ]; then
        menu "$NOMBRE ya está instalado. ¿Qué quieres hacer?" "Actualizar a la última versión" "Desinstalar" "Cancelar"
    fi
    case $ELEGIDA in 0) instalar ;; 1) desinstalar ;; *) cancelar ;; esac
else
    if [ -z "$VERSION" ]; then echo; error "no hay ningún release publicado todavía, o no hay conexión a internet"; fi
    ELEGIDA=0
    if [ $INTERACTIVO = 1 ]; then
        menu "¿Cómo quieres instalarlo?" "Instalar en $(corto "$DIR") (recomendado)" "Elegir otra carpeta" "Cancelar"
    fi
    case $ELEGIDA in 0) instalar ;; 1) pedir_carpeta; instalar ;; *) cancelar ;; esac
fi
