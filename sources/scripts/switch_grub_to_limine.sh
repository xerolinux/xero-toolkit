#!/bin/bash
set -e

TIMESTAMP=$(date +%Y.%m.%d-%H.%M.%S)
BACKUP_FILE="/root/grub-backup-${TIMESTAMP}.tar.gz"

# Locate the EFI System Partition mountpoint.
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

# Back up everything GRUB-related before touching anything.
BACKUP_PATHS=()
for path in /etc/default/grub /etc/grub.d /boot/grub "${ESP}/EFI/GRUB" "${ESP}/EFI/grub"; do
  [[ -e "$path" ]] && BACKUP_PATHS+=("$path")
done

if [[ ${#BACKUP_PATHS[@]} -gt 0 ]]; then
  tar czf "$BACKUP_FILE" "${BACKUP_PATHS[@]}"
  echo "GRUB backup saved to $BACKUP_FILE"
else
  echo "No GRUB files found to back up"
fi

# Capture GRUB's own EFI loader path(s) on the ESP before removing anything,
# so its stale NVRAM boot entry can be matched by content (not NVRAM label,
# which varies by installer) and removed further down.
GRUB_EFI_PATHS=""
if [[ -d "${ESP}/EFI" ]]; then
  GRUB_EFI_PATHS="$(find "${ESP}/EFI" -iname 'grubx64.efi' 2>/dev/null | sed "s#^${ESP}##")"
fi

# Remove grub and every package that depends on it, keeping only what's
# actually installed (pacman -Rns aborts entirely if any target is absent).
CANDIDATE_PKGS=(grub grub-hooks update-grub os-prober)
REMOVE_PKGS=()
for pkg in "${CANDIDATE_PKGS[@]}"; do
  if pacman -Q "$pkg" >/dev/null 2>&1; then
    REMOVE_PKGS+=("$pkg")
  fi
done
pacman -Rns --noconfirm "${REMOVE_PKGS[@]}"

# Clear leftover config/theme files pacman leaves as .pacsave or never owned.
rm -rf /etc/default/grub /etc/default/grub.pacsave /etc/grub.d /boot/grub \
  "${ESP}/EFI/GRUB" "${ESP}/EFI/grub"

# Keep the boot menu to just kernel entries and real other-OS entries -
# suppress limine-mkinitcpio-hook's own found-bootloader scan (systemd-boot/
# rEFInd/other EFI loader) and its extra UEFI fallback registration; other
# real OSes are added separately via chainload detection (efibootmgr scan).
cat > /etc/default/limine <<'EOF'
FIND_BOOTLOADERS=no
ENABLE_LIMINE_FALLBACK=no
EOF

# Deploy Limine to the ESP.
mkdir -p "${ESP}/EFI/limine"
cp "/usr/share/limine/BOOTX64.EFI" "${ESP}/EFI/limine/BOOTX64.EFI"

# Deploy fallback EFI boot binary for UEFI firmware default path.
mkdir -p "${ESP}/EFI/BOOT"
cp "/usr/share/limine/BOOTX64.EFI" "${ESP}/EFI/BOOT/BOOTX64.EFI"

# Register a boot entry via efibootmgr.
ESP_SOURCE=$(findmnt -n -o SOURCE "$ESP")
ESP_DISK=$(lsblk -no PKNAME "$ESP_SOURCE" | head -n1)
ESP_PART=$(lsblk -no PARTN "$ESP_SOURCE")

# Idempotent: skip creation if this exact loader path is already registered
# (matched by content, not label) - re-running this script must not pile up
# duplicate "Limine Bootloader" firmware boot entries.
if efibootmgr -v 2>/dev/null | grep -qi 'EFI\\limine\\BOOTX64\.EFI'; then
  echo "Limine EFI boot entry already registered, skipping."
elif [[ -n "$ESP_DISK" && -n "$ESP_PART" ]]; then
  efibootmgr --create --disk "/dev/${ESP_DISK}" --part "$ESP_PART" \
    --loader '\EFI\limine\BOOTX64.EFI' --label 'Limine Bootloader' --unicode
else
  echo "Could not determine ESP disk/partition for efibootmgr registration" >&2
  exit 1
fi

# Remove GRUB's now-stale EFI boot entry so it no longer clutters the
# firmware boot menu, now that Limine's own entry is registered above.
if [[ -n "$GRUB_EFI_PATHS" ]]; then
  EFIBOOTMGR_OUTPUT="$(efibootmgr -v 2>/dev/null)"
  while IFS= read -r relpath; do
    [[ -z "$relpath" ]] && continue
    winpath="${relpath//\//\\}"
    bootnum="$(grep -iF "$winpath" <<< "$EFIBOOTMGR_OUTPUT" | grep -oE '^Boot[0-9A-Fa-f]{4}' | head -n1 | sed 's/^Boot//')" || true
    if [[ -n "$bootnum" ]]; then
      efibootmgr -b "$bootnum" -B
      echo "Removed stale GRUB firmware boot entry Boot${bootnum}"
    fi
  done <<< "$GRUB_EFI_PATHS"
fi

echo "Limine installed. Run limine-mkinitcpio (or limine-update) to generate limine.conf entries."
