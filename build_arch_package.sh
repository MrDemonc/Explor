#!/usr/bin/env bash
# ==============================================================================
# Script de compilación y empaquetado para Arch Linux
# Proyecto: Explor (com.demonc.explor)
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

COLOR_BLUE="\033[1;34m"
COLOR_GREEN="\033[1;32m"
COLOR_YELLOW="\033[1;33m"
COLOR_RED="\033[1;31m"
COLOR_RESET="\033[0m"

log_info() {
    echo -e "${COLOR_BLUE}==>${COLOR_RESET} $1"
}

log_success() {
    echo -e "${COLOR_GREEN}==>${COLOR_RESET} $1"
}

log_warn() {
    echo -e "${COLOR_YELLOW}==>${COLOR_RESET} $1"
}

log_error() {
    echo -e "${COLOR_RED}==> ERROR:${COLOR_RESET} $1" >&2
}

INSTALL_AFTER_BUILD=false
CLEAN_BUILD=false

for arg in "$@"; do
    case "$arg" in
        -i|--install)
            INSTALL_AFTER_BUILD=true
            ;;
        -c|--clean)
            CLEAN_BUILD=true
            ;;
        -h|--help)
            echo "Uso: $0 [OPCIONES]"
            echo ""
            echo "Opciones:"
            echo "  -i, --install    Instala el paquete generado con pacman tras compilar"
            echo "  -c, --clean      Limpia temporales de empaquetado en packaging/arch/"
            echo "  -h, --help       Muestra esta ayuda"
            exit 0
            ;;
    esac
done

echo -e "${COLOR_BLUE}====================================================${COLOR_RESET}"
echo -e "${COLOR_BLUE}  Compilación y Empaquetado para Arch Linux - Explor ${COLOR_RESET}"
echo -e "${COLOR_BLUE}====================================================${COLOR_RESET}"

# 1. Comprobar herramientas necesarias
log_info "Verificando dependencias del sistema..."
for tool in cargo rsvg-convert makepkg; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        log_error "No se encontró el comando requerido: '$tool'. Por favor instálalo."
        exit 1
    fi
done

# 2. Generar iconos PNG a partir del SVG vectorial de carpeta
log_info "Generando iconos PNG en resoluciones estándar del sistema..."
ICON_SVG="assets/icons/com.demonc.explor.svg"
if [ ! -f "$ICON_SVG" ]; then
    log_error "No se encontró el archivo SVG del icono: $ICON_SVG"
    exit 1
fi

SIZES=(16 24 32 48 64 128 256 512)
for size in "${SIZES[@]}"; do
    DEST_DIR="assets/icons/hicolor/${size}x${size}/apps"
    mkdir -p "$DEST_DIR"
    rsvg-convert -w "$size" -h "$size" "$ICON_SVG" -o "$DEST_DIR/com.demonc.explor.png"
done
log_success "Iconos generados exitosamente (16px a 512px + SVG)."

# 3. Compilar binario optimizado con Cargo
log_info "Compilando Explor en modo release (optimizado)..."
cargo build --release
log_success "Binario compilado: target/release/explor"

# 4. Empaquetar con makepkg en directorio aislado packaging/arch/
log_info "Construyendo paquete .pkg.tar.zst para Arch Linux..."
PKG_DIR="$SCRIPT_DIR/packaging/arch"

if [ ! -d "$PKG_DIR" ] || [ ! -f "$PKG_DIR/PKGBUILD" ]; then
    log_error "No se encontró $PKG_DIR/PKGBUILD"
    exit 1
fi

# Limpieza segura de los temporales de makepkg ÚNICAMENTE dentro de packaging/arch
rm -rf "$PKG_DIR/pkg" "$PKG_DIR/src"

# Ejecutar makepkg dentro de packaging/arch
(
    cd "$PKG_DIR"
    makepkg -f --nodeps
)

# Mover el paquete generado al directorio raíz del proyecto
GENERATED_PKG=$(ls -t "$PKG_DIR"/explor-*-x86_64.pkg.tar.zst 2>/dev/null | head -n 1)

if [ -z "$GENERATED_PKG" ] || [ ! -f "$GENERATED_PKG" ]; then
    log_error "No se pudo encontrar el archivo .pkg.tar.zst generado en $PKG_DIR."
    exit 1
fi

FINAL_PKG="$SCRIPT_DIR/$(basename "$GENERATED_PKG")"
cp -f "$GENERATED_PKG" "$FINAL_PKG"

# Limpieza dentro de packaging/arch
rm -rf "$PKG_DIR/pkg" "$PKG_DIR/src" "$PKG_DIR"/*.pkg.tar.zst

PKG_SIZE=$(du -h "$FINAL_PKG" | cut -f1)
log_success "¡Paquete creado exitosamente!"
echo -e "   ${COLOR_GREEN}Archivo:${COLOR_RESET} $FINAL_PKG ($PKG_SIZE)"

# 5. Limpieza opcional
if [ "$CLEAN_BUILD" = true ]; then
    log_info "Limpieza completada."
fi

# 6. Instalación opcional
if [ "$INSTALL_AFTER_BUILD" = true ]; then
    echo ""
    log_info "Instalando el paquete con pacman (requiere permisos sudo)..."
    sudo pacman -U --noconfirm "$FINAL_PKG"
    log_success "¡Explor ha sido instalado en tu sistema Arch Linux!"
    echo -e "   Puedes ejecutarlo desde el menú de aplicaciones o con el comando: ${COLOR_GREEN}explor${COLOR_RESET}"
else
    echo ""
    echo -e "${COLOR_YELLOW}Para instalar el paquete generado ejecuta:${COLOR_RESET}"
    echo -e "   ${COLOR_GREEN}sudo pacman -U $FINAL_PKG${COLOR_RESET}"
    echo -e "O puedes volver a ejecutar este script con la opción ${COLOR_GREEN}--install${COLOR_RESET}:"
    echo -e "   ${COLOR_GREEN}./build_arch_package.sh --install${COLOR_RESET}"
fi

echo -e "${COLOR_BLUE}====================================================${COLOR_RESET}"
