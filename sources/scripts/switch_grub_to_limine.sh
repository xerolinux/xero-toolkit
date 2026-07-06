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

# Remove grub, and os-prober/update-grub only if actually installed.
REMOVE_PKGS=(grub)
for pkg in os-prober update-grub; do
  if pacman -Q "$pkg" >/dev/null 2>&1; then
    REMOVE_PKGS+=("$pkg")
  fi
done
pacman -Rns --noconfirm "${REMOVE_PKGS[@]}"

# Clear leftover config/theme files pacman leaves as .pacsave or never owned.
rm -rf /etc/default/grub /etc/default/grub.pacsave /etc/grub.d /boot/grub \
  "${ESP}/EFI/GRUB" "${ESP}/EFI/grub"

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

if [[ -n "$ESP_DISK" && -n "$ESP_PART" ]]; then
  efibootmgr --create --disk "/dev/${ESP_DISK}" --part "$ESP_PART" \
    --loader '\EFI\limine\BOOTX64.EFI' --label 'Limine Bootloader' --unicode
else
  echo "Could not determine ESP disk/partition for efibootmgr registration" >&2
  exit 1
fi

echo "Limine installed. Run mkinitcpio -P to generate limine.conf entries."
