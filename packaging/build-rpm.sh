#!/bin/bash
# Сборка RPM-пакета AnsibleOps в окружении РЕДОС 7 (или любом EL7-совместимом).
#
# Использование (внутри контейнера РЕДОС):
#   bash packaging/build-rpm.sh /path/to/источник [rpmbuild_topdir]
#
# Скрипт:
#   1. готовит rpmbuild-каталоги (%{_topdir}),
#   2. собирает исходный tarball ansibleops-<version>.tar.gz,
#   3. запускает rpmbuild -bb packaging/ansibleops.spec,
#   4. копирует готовый RPM и SHA256SUMS в ./dist.
#
# Скрипт сам нормализует переводы строк (CRLF -> LF) и перезапускает себя,
# поэтому его безопасно вызывать прямо из Windows-источника.
if [ -n "${BASH_VERSION:-}" ] && head -c 200 "$0" | grep -q $'\r'; then
  _br_tmp="$(mktemp)"
  tr -d '\r' < "$0" > "$_br_tmp"
  exec bash "$_br_tmp" "$@"
fi
set -euo pipefail

SRC_DIR="${1:-$(cd "$(dirname "$0")/.." && pwd)}"
TOPDIR="${2:-/root/rpmbuild}"
DIST_DIR="${SRC_DIR}/dist"
PKG_NAME="ansibleops"

cd "$SRC_DIR"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
RELEASE="1"
if [ -z "$VERSION" ]; then
  echo "ERROR: не удалось определить версию в Cargo.toml" >&2
  exit 1
fi
echo "==> AnsibleOps ${VERSION}-${RELEASE}"

# --- 1. rpmbuild topdir ---
mkdir -p "$TOPDIR"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
chmod 755 "$TOPDIR" "$TOPDIR"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}

# --- 2. исходный tarball ---
TARBALL="${PKG_NAME}-${VERSION}.tar.gz"
rm -f "$TOPDIR/SOURCES/$TARBALL"

# staging-копия: нормализуем переводы строк (Windows -> Unix),
# иначе rpmbuild падает на \r в .spec/.sh
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
mkdir -p "$STAGE/${PKG_NAME}-${VERSION}"
tar --exclude='./target' --exclude='./.git' --exclude='./dist' \
    --exclude='./.github' --exclude='./.cargo-home' \
    --exclude='./_o1.txt' \
    -cf - . | tar -C "$STAGE/${PKG_NAME}-${VERSION}" -xf -
find "$STAGE" -type f \( -name '*.sh' -o -name '*.spec' -o -name '*.yml' -o -name '*.yaml' -o -name '*.toml' -o -name '*.md' \) \
    -exec sed -i 's/\r$//' {} +
tar -czf "$TOPDIR/SOURCES/$TARBALL" -C "$STAGE" "${PKG_NAME}-${VERSION}"
echo "==> tarball: $TOPDIR/SOURCES/$TARBALL"

# --- 3. сборка RPM ---
cp packaging/ansibleops.spec "$TOPDIR/SPECS/ansibleops.spec"
sed -i 's/\r$//' "$TOPDIR/SPECS/ansibleops.spec"
# синхронизируем версию из Cargo.toml в spec
sed -i "s/^%global ansibleops_version .*/%global ansibleops_version ${VERSION}/" "$TOPDIR/SPECS/ansibleops.spec"

rpmbuild --define "_topdir $TOPDIR" \
         -bb "$TOPDIR/SPECS/ansibleops.spec"

# --- 4. результат ---
mkdir -p "$DIST_DIR"
RPM_FILE="$(find "$TOPDIR/RPMS" -name "${PKG_NAME}-${VERSION}-*.rpm" | head -n1)"
if [ -z "$RPM_FILE" ]; then
  echo "ERROR: RPM не собран" >&2
  exit 1
fi
cp "$RPM_FILE" "$DIST_DIR/"
cp "$TOPDIR/SOURCES/$TARBALL" "$DIST_DIR/" 2>/dev/null || true

cd "$DIST_DIR"
sha256sum ./*.rpm ./*.tar.gz > SHA256SUMS 2>/dev/null || sha256sum ./*.rpm > SHA256SUMS

echo "==> Готово:"
ls -la "$DIST_DIR"
cat "$DIST_DIR/SHA256SUMS"
