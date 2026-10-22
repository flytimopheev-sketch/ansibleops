#!/bin/bash
# Нормализация переводов строк: если файл пришёл с CRLF (Windows checkout),
# перезапускаем себя уже в LF-версии. Guard намеренно собран в одну строку
# с завершающим комментарием — иначе CR в конце строк ломает синтаксис
# самого guard'а (`then\r` / `fi\r` не распознаются bash'ом).
# Используется временный файл, а не <(...): process substitution не seekабелен,
# и bash при повторном чтении скрипта теряет смещение и «съедает» символы.
if [ -n "${BASH_VERSION:-}" ] && head -c 200 "$0" | grep -q $'\r'; then _ci_tmp="$(mktemp)"; tr -d '\r' < "$0" > "$_ci_tmp"; exec bash "$_ci_tmp" "$@"; fi # crlf-guard
set -euxo pipefail

# ---------------------------------------------------------------------------
# Подготовка окружения РЕДОС 7 для сборки RPM (AnsibleOps).
#
# Ключевая проблема образа alrdockerhub/redos:
#   * базовый rpm — 4.13.90, а БД /var/lib/rpm — Berkeley DB (bdb);
#   * rpm 4.13.90 НЕ поддерживает макросы вида `%[ "a" == "b" ? "x" : "y" ]`;
#   * но `redos-rpm-config` и `rust-srpm-macros` (обязательные для rpmbuild)
#     содержат такие макросы → rpmbuild падает на %prep.
#
# Решение: обновить rpm до 4.18.x (поддерживает `%[ ]`), а БД мигрировать
# из bdb в SQLite. Поведение `rpm --rebuilddb` зависит от ФС: на обычной ФС
# он сам подменяет /var/lib/rpm (и каталога rpmrebuilddb.* не остаётся),
# а на overlayfs подмена падает с EXDEV (Invalid cross-device link) и
# готовую БД нужно скопировать вручную. Оба сценария обработаны в шаге 3.
# ---------------------------------------------------------------------------

RPMDB_DIR=/var/lib/rpm

# --- 1. Репозитории -------------------------------------------------------
# В образе alrdockerhub/redos baseurl перечисляет 3 зеркала, и
# первым идёт https://repo1.red-soft.ru — но в системном
# CA-bundle образа нет российских корневых сертификатов, поэтому
# TLS-соединение с ним падает и dnf долго ждёт таймаут.
# Переставляем зеркала так, чтобы первым шло рабочее HTTPS-зеркало
# Яндекса (его сертификат проверяется штатным CA-bundle),
# а http://repo.red-soft.ru остаётся запасным вариантом.
# GPG-проверка пакетов не отключается (ключи локальные,
# file:///etc/pki/rpm-gpg/RPM-GPG-KEY-RED-SOFT).
for repo in /etc/yum.repos.d/RedOS-Base.repo /etc/yum.repos.d/RedOS-Updates.repo; do
  [ -f "$repo" ] || continue
  sed -i -E \
    -e 's#^baseurl=https?://repo1?\.red-soft\.ru/redos/7\.3/\$basearch/os.*#baseurl=https://mirror.yandex.ru/redos/7.3/$basearch/os,http://repo.red-soft.ru/redos/7.3/$basearch/os#' \
    -e 's#^baseurl=https?://repo1?\.red-soft\.ru/redos/7\.3/\$basearch/updates.*#baseurl=https://mirror.yandex.ru/redos/7.3/$basearch/updates,http://repo.red-soft.ru/redos/7.3/$basearch/updates#' \
    "$repo"
done
grep -H '^baseurl' /etc/yum.repos.d/*.repo

# --- 2. Обновление rpm ----------------------------------------------------
# rpm 4.18 нужен для поддержки макросов `%[ ... ]` (их используют
# redos-rpm-config и rust-srpm-macros, без них rpmbuild падает на %prep).
# rpm и rpm-libs обновляем одной транзакцией — они должны совпадать по версии.
# На этом шаге dnf ещё работает: текущий rpm 4.13.90 умеет писать в bdb.
# Проблема возникнет позже — после обновления rpm БД окажется в режиме
# "только чтение" (bdb_ro), что и чинится в шаге 3.
dnf -y update rpm rpm-libs
rpm --version

# `rpm --rebuilddb` конвертирует БД в sqlite, но его поведение зависит
# от ФС: он делает rename("/var/lib/rpm" -> "/var/lib/rpmold.N") с последующей
# подменой каталога, а на overlayfs это даёт EXDEV (cross-device link).
# Поэтому возможны два исхода:
#   A) подмена удалась (обычная ФС в CI) - БД уже sqlite, делать нечего;
#   B) подмена не удалась (overlayfs) - rpm оставляет готовую БД в
#      /var/lib/rpmrebuilddb.<pid>/, её нужно скопировать вручную.
# Оба случая обрабатываются ниже.
if [ -f "$RPMDB_DIR/Packages" ]; then
  # rebuilddb может завершиться с ошибкой (случай B) - это не фатально.
  rpm --rebuilddb || true

  if [ -f "$RPMDB_DIR/rpmdb.sqlite" ]; then
    # Случай A: БД уже сконвертирована и подменена на месте.
    echo "rpmdb: конвертация выполнена на месте (rpmdb.sqlite)"
  else
    # Случай B: забираем БД из каталога, который оставил rebuilddb.
    REBUILD_DIR="$(ls -d /var/lib/rpmrebuilddb.* 2>/dev/null | sort -V | tail -n1)"
    if [ -z "$REBUILD_DIR" ] || [ ! -f "$REBUILD_DIR/rpmdb.sqlite" ]; then
      echo "ERROR: не удалось сконвертировать БД rpm в sqlite" >&2
      exit 1
    fi
    echo "rpmdb: копирование БД из $REBUILD_DIR"
    cp -f "$REBUILD_DIR"/rpmdb.sqlite "$RPMDB_DIR"/rpmdb.sqlite
    # WAL/SHM могут отсутствовать - под set -e падение [ -f ] недопустимо.
    [ -f "$REBUILD_DIR/rpmdb.sqlite-shm" ] && \
      cp -f "$REBUILD_DIR/rpmdb.sqlite-shm" "$RPMDB_DIR"/rpmdb.sqlite-shm || true
    [ -f "$REBUILD_DIR/rpmdb.sqlite-wal" ] && \
      cp -f "$REBUILD_DIR/rpmdb.sqlite-wal" "$RPMDB_DIR"/rpmdb.sqlite-wal || true
  fi

  rm -rf /var/lib/rpmrebuilddb.* /var/lib/rpmold.*
  # Удаляем legacy bdb-индексы, иначе rpm продолжит открывать bdb_ro.
  rm -f "$RPMDB_DIR"/Packages "$RPMDB_DIR"/Basenames "$RPMDB_DIR"/Conflictname \
        "$RPMDB_DIR"/Dirnames "$RPMDB_DIR"/Enhancename "$RPMDB_DIR"/Filetriggername \
        "$RPMDB_DIR"/Group "$RPMDB_DIR"/Installtid "$RPMDB_DIR"/Name \
        "$RPMDB_DIR"/Obsoletename "$RPMDB_DIR"/Providename \
        "$RPMDB_DIR"/Recommendname "$RPMDB_DIR"/Requirename \
        "$RPMDB_DIR"/Sha1header "$RPMDB_DIR"/Sigmd5 "$RPMDB_DIR"/Suggestname \
        "$RPMDB_DIR"/Supplementname "$RPMDB_DIR"/Transfiletriggername \
        "$RPMDB_DIR"/Triggername "$RPMDB_DIR"/__db.*
fi
rm -f "$RPMDB_DIR"/.rebuilddb

# Итоговая проверка: БД обязана быть в sqlite-формате, иначе dnf не работает.
if [ ! -f "$RPMDB_DIR/rpmdb.sqlite" ]; then
  echo "ERROR: БД rpm не переведена в sqlite" >&2
  exit 1
fi

# --- 4. Проверка состояния БД ---------------------------------------------
rpm -qa | wc -l
# Транзакция dnf должна проходить: если БД не мигрирована — упадёт.
dnf -y install wget
rpm -q wget

# --- 5. Установка зависимостей ---------------------------------------------
# ВАЖНО: всё ставим ОДНОЙ транзакцией.
#  * binutils нужен >= 2.35 (иначе `as` не понимает --gdwarf-5,
#    который требует rustc 1.91 для crate `ring`).
#  * cargo/rust 1.91.1 нужны для edition 2021 и свежих crates.
dnf -y install \
  ca-certificates-ru \
  git tar gzip which findutils \
  rpm-build binutils rust-srpm-macros \
  cargo rust \
  gcc gcc-c++ make pkgconfig \
  libX11-devel libxcb-devel mesa-libGL-devel oniguruma-devel

# --- 6. Проверка инструментов ----------------------------------------------
rpm -q rpm rpm-libs rpm-build redos-rpm-config
case "$(rpm -q rpm)" in
  *4.1[7-9].*) ;;
  *) echo "ERROR: ожидался rpm 4.17+ (нужна поддержка макросов %[ ])" >&2; exit 1 ;;
esac
for t in git tar gzip rpmbuild cargo rustc gcc make as; do
  command -v "$t" >/dev/null 2>&1 || { echo "ERROR: $t not available" >&2; exit 1; }
done
cargo --version
rustc --version
as --version | head -n1