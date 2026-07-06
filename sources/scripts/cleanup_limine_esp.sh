#!/bin/bash
# Prunes ESP clutter Limine no longer needs, keeping usage at a minimum on a
# space-limited EFI system partition. Ported from
# ~/xwork/LimineSecureBoot/lib/migrate.sh's cleanup_after_migrate() and its
# helpers. Best-effort: never fails the overall switch, since by the time
# this runs Limine is already installed and working.
set +e

ESP=""
for candidate in /boot/efi /efi /boot; do
  if mountpoint -q "$candidate" && [[ "$(findmnt -no FSTYPE "$candidate")" == "vfat" ]]; then
    ESP="$candidate"
    break
  fi
done

if [[ -z "$ESP" ]]; then
  echo "Could not locate EFI System Partition mountpoint; skipping ESP cleanup." >&2
  exit 0
fi

LIMINE_CONF="${ESP}/limine.conf"

# Prune kernel/initramfs images and UKIs in the ESP that limine.conf no
# longer references. Safety model: limine.conf is authoritative - only
# delete ESP files under a <machine-id>/ dir (or EFI/Linux/*.efi) NOT
# referenced by a path:/module_path: line. If no referenced path parses at
# all, delete nothing.
if [[ -f "$LIMINE_CONF" ]]; then
  REFERENCED=()
  while IFS= read -r line; do
    p="${line#*boot():}"
    p="${p%%#*}"
    p="${p#"${p%%[![:space:]]*}"}"
    p="${p%"${p##*[![:space:]]}"}"
    [[ -n "$p" ]] && REFERENCED+=("$p")
  done < <(grep -E '(module_)?path:[[:space:]]*boot\(\):' "$LIMINE_CONF" 2>/dev/null)

  if [[ ${#REFERENCED[@]} -gt 0 ]]; then
    for mid_dir in "${ESP}"/*/; do
      mid_dir="${mid_dir%/}"
      mid="${mid_dir##*/}"
      # Only machine-id dirs (32 lowercase hex chars) hold Limine's kernels.
      [[ "$mid" =~ ^[0-9a-f]{32}$ ]] || continue
      while IFS= read -r kfile; do
        [[ -n "$kfile" ]] || continue
        relpath="${kfile#"$ESP"}"
        is_referenced=0
        for ref in "${REFERENCED[@]}"; do
          [[ "$ref" == "$relpath" ]] && { is_referenced=1; break; }
        done
        [[ "$is_referenced" == "1" ]] && continue
        rm -f "$kfile"
        echo "Removed orphaned ESP kernel file: $relpath"
      done < <(find "$mid_dir" -type f 2>/dev/null)
      find "$mid_dir" -type d -empty -delete 2>/dev/null
    done
  else
    echo "Skipping orphaned-kernel prune: no referenced kernels found in $LIMINE_CONF."
  fi

  # Unified Kernel Images (EFI/Linux/*.efi), only relevant when ENABLE_UKI is
  # in use. Only deletes a UKI limine.conf doesn't reference, and only when
  # it references at least one EFI/Linux UKI at all.
  UKI_DIR="${ESP}/EFI/Linux"
  if [[ -d "$UKI_DIR" ]]; then
    UKI_REFERENCED=()
    while IFS= read -r line; do
      p="${line#*boot():}"
      p="${p%%#*}"
      p="${p#"${p%%[![:space:]]*}"}"
      p="${p%"${p##*[![:space:]]}"}"
      [[ "$p" == /EFI/Linux/*.efi ]] && UKI_REFERENCED+=("$p")
    done < <(grep -iE '(module_)?path:[[:space:]]*boot\(\):.*/EFI/Linux/.*\.efi' "$LIMINE_CONF" 2>/dev/null)

    if [[ ${#UKI_REFERENCED[@]} -gt 0 ]]; then
      while IFS= read -r ukifile; do
        [[ -n "$ukifile" ]] || continue
        relpath="${ukifile#"$ESP"}"
        is_referenced=0
        for ref in "${UKI_REFERENCED[@]}"; do
          [[ "$ref" == "$relpath" ]] && { is_referenced=1; break; }
        done
        [[ "$is_referenced" == "1" ]] && continue
        rm -f "$ukifile"
        echo "Removed orphaned UKI: $relpath"
      done < <(find "$UKI_DIR" -maxdepth 1 -type f -iname '*.efi' 2>/dev/null)
    fi
  fi
fi

# Remove backup files Limine's own tooling leaves behind.
[[ -e "${ESP}/limine.conf.old" ]] && rm -f "${ESP}/limine.conf.old"
if [[ -d "${ESP}/EFI/limine" ]]; then
  while IFS= read -r f; do
    [[ -n "$f" ]] && rm -f "$f"
  done < <(find "${ESP}/EFI/limine" -maxdepth 1 -name '*.bak' 2>/dev/null)
fi

# Leftovers from bootloaders that used to own this ESP.
if [[ -d "${ESP}/loader" && ! -e "${ESP}/EFI/systemd/systemd-bootx64.efi" ]]; then
  rm -rf "${ESP}/loader"
  echo "Removed unused systemd-boot loader/ directory."
fi
for d in "${ESP}/EFI/grub" "${ESP}/EFI/GRUB"; do
  [[ -d "$d" ]] || continue
  rmdir "$d" 2>/dev/null
done

echo "ESP cleanup complete."
exit 0
