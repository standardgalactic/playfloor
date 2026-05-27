#!/usr/bin/env bash
# ============================================================
#  build.sh — Build Semantic Infrastructure monograph
#  Usage: ./build.sh [--clean] [--draft]
# ============================================================
set -euo pipefail

MAIN="main"
BUILDDIR="build"
TEXMFCNF_OVERRIDE="$(pwd)/${BUILDDIR}:"

mkdir -p "$BUILDDIR"
# Local texmf.cnf to increase input stack for complex tikz diagrams
cat > "${BUILDDIR}/texmf.cnf" << 'CNFEOF'
stack_size = 200000
CNFEOF

export TEXMFCNF="${TEXMFCNF_OVERRIDE}"
export TEXINPUTS=".:./styles:./chapters:./diagrams:"

case "${1:-}" in
  --clean)
    echo "Cleaning..."
    rm -rf "${BUILDDIR}"
    mkdir -p "${BUILDDIR}"
    ;;
  --draft)
    echo "Draft compile (1 pass, no biber)..."
    lualatex -interaction=nonstopmode -output-directory="${BUILDDIR}" "${MAIN}.tex"
    cp "${BUILDDIR}/${MAIN}.pdf" "./${MAIN}.pdf"
    echo "Done: ${MAIN}.pdf"
    ;;
  *)
    echo "Full build (lualatex → biber → lualatex × 2)..."
    lualatex -interaction=nonstopmode -output-directory="${BUILDDIR}" "${MAIN}.tex"
    biber --input-directory=. "${BUILDDIR}/${MAIN}"
    lualatex -interaction=nonstopmode -output-directory="${BUILDDIR}" "${MAIN}.tex"
    lualatex -interaction=nonstopmode -output-directory="${BUILDDIR}" "${MAIN}.tex"
    cp "${BUILDDIR}/${MAIN}.pdf" "./${MAIN}.pdf"
    echo "Done: ${MAIN}.pdf ($(pdfinfo ${MAIN}.pdf 2>/dev/null | grep Pages | awk '{print $2}') pages)"
    ;;
esac
