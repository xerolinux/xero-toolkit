#!/bin/bash
set -e

ESP=""
for candidate in /boot/efi /efi /boot; do
  if mountpoint -q "$candidate" && [[ "$(findmnt -no FSTYPE "$candidate")" == "vfat" ]]; then
    ESP="$candidate"
    break
  fi
done

if [[ -z "$ESP" ]]; then
  echo "Could not locate EFI System Partition mountpoint" >&2
  exit 1
fi

LIMINE_CONF="${ESP}/limine.conf"

if [[ ! -f "$LIMINE_CONF" ]]; then
  echo "limine.conf not found at $LIMINE_CONF" >&2
  exit 1
fi

# Non-OS firmware boot entries to exclude, grouped by CATEGORY rather than
# vendor phrase, so this covers any manufacturer instead of chasing one
# vendor's exact wording.
NON_OS_PATTERNS=(
  'BootManagerMenu' 'Boot Menu' 'Boot Device Menu' 'EFI Firmware Setup'
  'UEFI Misc Device' 'UEFI Shell' 'Internal Shell' 'Enter Setup' 'Setup'
  'CSM' 'Launch CSM' '\<UEFI OS\>'
  'Diagnostic' 'SupportAssist' 'Hardware Diagnostics'
  'Diskette' 'Floppy' '\<FDD[0-9]*\>' '\<HDD[0-9]*\>' '\<SSD[0-9]*\>'
  '\<SATA[0-9]*\>' '\<ATA[0-9]*\>' 'ATAPI' '\<IDE\>' '\<NVMe[0-9]*\>'
  '\<eMMC\>' 'CD[- /]?ROM' 'DVD[- /]?ROM' 'Optical Drive'
  'USB (HDD|FDD|CD|DVD|Storage|Flash|Disk|Boot)'
  '\<NIC\>' 'Network (Adapter|Boot|Card)' 'Onboard NIC' 'PXEv4' 'PXEv6'
  'HTTPv4' 'HTTPv6' 'LAN Boot' 'PCI LAN'
)
EXCLUDE_PATTERN="$(IFS='|'; printf '%s' "${NON_OS_PATTERNS[*]}")"

# Detect real other-OS firmware entries: every real OS/bootloader entry
# references a disk partition via an HD(...) device-path node (Windows Boot
# Manager is special-cased via VenHw(...)). Excludes XeroLinux/Limine's own
# entry and every non-OS category above.
OTHER_OS="$(efibootmgr -v 2>/dev/null \
  | grep -E '^Boot[0-9A-Fa-f]{4}\*?[[:space:]]' \
  | grep -viE 'xerolinux|limine' \
  | grep -viE "$EXCLUDE_PATTERN" \
  | grep -E 'Windows Boot Manager|HD\(' \
  | sed -E 's/^Boot[0-9A-Fa-f]{4}\*?[[:space:]]+//' \
  | sed -E 's/\t.*$//' \
  || true)"

if [[ -z "$OTHER_OS" ]]; then
  echo "No other OS entries detected; nothing to chainload."
  exit 0
fi

chainload_loader_path_for_os() {
  case "$1" in
    *[Ww]indows*) printf '/EFI/Microsoft/Boot/bootmgfw.efi' ;;
    *) printf '/EFI/Boot/bootx64.efi' ;;
  esac
}

# Idempotent: strip any chainload block this script previously appended
# (always the last thing in the file) before re-appending, so reruns don't
# duplicate entries.
TMP=$(mktemp)
awk '/^### XeroLinux Chainload/ { exit } { print }' "$LIMINE_CONF" > "$TMP"

{
  cat "$TMP"
  echo ""
  echo "### XeroLinux Chainload"
  while IFS= read -r os_label; do
    [[ -z "$os_label" ]] && continue
    loader_path="$(chainload_loader_path_for_os "$os_label")"
    echo ""
    echo "/$os_label"
    echo "    protocol: efi_chainload"
    echo "    path: boot():${loader_path}"
  done <<< "$OTHER_OS"
} > "${LIMINE_CONF}.new"
rm -f "$TMP"
mv "${LIMINE_CONF}.new" "$LIMINE_CONF"

echo "Chainload entries added for other detected OS(es)."
