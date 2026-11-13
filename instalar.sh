#!/usr/bin/env bash
# Instalador de Logic_F-4:13 para Linux y macOS:
#   curl -fsSL https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/VstevenMata/New-Logic-IDE/main/instalar.sh | bash -s -- -y
set -euo pipefail

REPO=VstevenMata/New-Logic-IDE
NOMBRE=logicf4-ide
VERSION=1.0.0

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
    B=$'\e[1m' R=$'\e[0m' VERDE=$'\e[32m' AMARILLO=$'\e[33m' ROJO=$'\e[31m'
else
    B='' R='' VERDE='' AMARILLO='' ROJO=''
fi
info() { printf '%sinfo:%s %s\n' "$VERDE$B" "$R" "$*"; }
aviso() { printf '%saviso:%s %s\n' "$AMARILLO$B" "$R" "$*" >&2; }
error() { printf '%serror:%s %s\n' "$ROJO$B" "$R" "$*" >&2; exit 1; }

uso() {
    cat <<EOF
$NOMBRE-installer $VERSION
Instala Logic_F-4:13 desde el último release de github.com/$REPO

uso: instalar.sh [opciones]

  -y, --yes        instala sin preguntar
      --dir RUTA   carpeta de destino (por defecto ~/.local/bin)
  -h, --help       muestra esta ayuda
EOF
}

SI=0 DIR="$HOME/.local/bin"
while [ $# -gt 0 ]; do
    case $1 in
        -y | --yes) SI=1 ;;
        --dir) DIR=${2:?falta la ruta después de --dir}; shift ;;
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
ARCHIVO="$NOMBRE-$SO-$ARQ"
URL="https://github.com/$REPO/releases/latest/download/$ARCHIVO"
APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"

# ~ y rutas relativas a absolutas, sin / final
normalizar() {
    DIR=${DIR/#\~/$HOME}
    case $DIR in /*) ;; *) DIR="$PWD/$DIR" ;; esac
    [ "$DIR" = / ] || DIR=${DIR%/}
}
escribible() { mkdir -p "$DIR" 2>/dev/null && [ -w "$DIR" ]; }

resumen() {
    printf '\n%s%s-installer%s %s\n\n' "$B" "$NOMBRE" "$R" "$VERSION"
    printf '  %-9s %s\n' sistema "$SO $ARQ" destino "${DIR/#$HOME/\~}/$NOMBRE"
    if [ $SO = linux ]; then printf '  %-9s %s\n' acceso "${APPS/#$HOME/\~}/$NOMBRE.desktop"; fi
    echo
}

normalizar
resumen

# Con curl | bash la entrada es el script: las respuestas se leen de la terminal; sin terminal, se instala sin preguntar
if [ $SI = 0 ] && ! { exec 3</dev/tty; } 2>/dev/null; then SI=1; fi
while [ $SI = 0 ]; do
    printf '  1) Instalar (por defecto)\n  2) Cambiar carpeta de destino\n  3) Cancelar\n%s>%s ' "$B" "$R"
    read -r op <&3 || op=3
    case ${op:-1} in
        1) if escribible; then echo; break; fi; aviso "no se puede escribir en $DIR" ;;
        2)
            printf 'Carpeta de destino [%s]: ' "${DIR/#$HOME/\~}"
            read -r nueva <&3 || nueva=""
            DIR=${nueva:-$DIR}; normalizar; resumen
            ;;
        3) echo "Instalación cancelada."; exit 0 ;;
        *) aviso "opción no válida: $op" ;;
    esac
done
escribible || error "no se puede escribir en $DIR"

DESTINO="$DIR/$NOMBRE"
TMP="$DESTINO.tmp"
trap 'rm -f "$TMP"' EXIT
info "descargando $ARCHIVO desde github.com/$REPO"
if [ -t 2 ]; then PROGRESO=--progress-bar; else PROGRESO=-s; fi
curl -fSL $PROGRESO --connect-timeout 10 -o "$TMP" "$URL" ||
    error "no se pudo descargar $URL (¿hay internet? ¿ya hay un release publicado?)"
chmod +x "$TMP"
mv "$TMP" "$DESTINO"
info "instalado en $DESTINO"

NUEVA_TERMINAL=0
case ":$PATH:" in
    *":$DIR:"*) info "$DIR ya está en el PATH" ;;
    *)
        NUEVA_TERMINAL=1
        linea="export PATH=\"$DIR:\$PATH\""
        case ${SHELL##*/} in bash) touch "$HOME/.bashrc" ;; zsh) touch "$HOME/.zshrc" ;; esac
        for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
            if [ -f "$rc" ] && ! grep -qF "$linea" "$rc"; then echo "$linea" >>"$rc"; fi
        done
        if command -v fish >/dev/null; then
            mkdir -p "$HOME/.config/fish/conf.d"
            echo "fish_add_path '$DIR'" >"$HOME/.config/fish/conf.d/$NOMBRE.fish"
        fi
        info "$DIR agregado al PATH"
        ;;
esac

if [ $SO = linux ]; then
    mkdir -p "$APPS"
    cat >"$APPS/$NOMBRE.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Logic_F-4:13
Comment=Diseña, visualiza y simula circuitos digitales
Exec="$DESTINO"
Terminal=false
Categories=Development;Education;Electronics;
EOF
    info "acceso agregado al menú de aplicaciones"
fi

printf '\n  %sLogic_F-4:13 se instaló correctamente.%s\n' "$VERDE$B" "$R"
printf '  Ejecuta %s%s%s para empezar.\n' "$B" "$NOMBRE" "$R"
if [ $NUEVA_TERMINAL = 1 ]; then printf '  Abre una terminal nueva para que reconozca el comando.\n'; fi
echo
