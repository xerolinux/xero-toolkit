#!/bin/bash
set -e

WALLPAPER_SRC="/usr/share/wallpapers/Xero-Plasma4.png"

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

if [[ ! -f "$WALLPAPER_SRC" ]]; then
  echo "XeroLinux wallpaper not found at $WALLPAPER_SRC" >&2
  exit 1
fi

cp "$WALLPAPER_SRC" "${ESP}/splash.png"

# Idempotently rewrite the ### Theme section to contain only the wallpaper
# directive, replacing (not appending to) any prior section.
TMP=$(mktemp)
awk '
  /^### Theme/ { skip=1; next }
  skip && (/^###/ || NF==0) { skip=0 }
  skip { next }
  { print }
' "$LIMINE_CONF" | sed '/./,$!d' > "$TMP"
{ printf '### Theme\nwallpaper: boot():/splash.png\n\n'; cat "$TMP"; } > "${LIMINE_CONF}.new"
rm -f "$TMP"
mv "${LIMINE_CONF}.new" "$LIMINE_CONF"

echo "Limine boot splash wallpaper applied."
