set -e
if [[ "$1" == "--flash" ]]; then
    cargo build --release --target x86_64-unknown-uefi
else
    cargo build --target x86_64-unknown-uefi
fi
mkdir -p esp/efi/boot
cp target/x86_64-unknown-uefi/debug/first-uefi-rust.efi esp/efi/boot/bootx64.efi
cp /usr/share/edk2/x64/OVMF_CODE.4m.fd .
cp /usr/share/edk2/x64/OVMF_VARS.4m.fd .

if [[ "$1" == "--img" || "$1" == "--iso" || "$1" == "--flash" ]]; then
    dd if=/dev/zero of=fat.img bs=1k count=1440
    mformat -i fat.img -f 1440 ::
    mmd -i fat.img ::/EFI
    mmd -i fat.img ::/EFI/BOOT
    mcopy -i fat.img esp/efi/boot/bootx64.efi ::/EFI/BOOT
fi

if [[ "$1" == "--iso" || "$1" == "--flash" ]]; then
    mkdir -p iso
    cp fat.img iso
    xorriso -as mkisofs -R -f -e fat.img -no-emul-boot -o cdimage.iso iso
fi

if [[ "$1" == "--flash" ]]; then
    echo "Available drives:"
    lsblk -d -o NAME,SIZE,MODEL
    echo "WARNING: This will overwrite the selected drive. Proceed with caution!"
    read -p "Enter the drive to flash to (e.g., /dev/sdX): " drive
    if [[ -n "$drive" ]]; then
        sudo dd if=cdimage.iso of="$drive" bs=4M status=progress oflag=sync
        echo "Flashing completed."
    else
        echo "No drive selected. Aborting."
    fi
fi

if [[ "$1" == "--iso" && "$2" == "--run" ]]; then
    cp /usr/share/edk2/x64/OVMF_CODE.4m.fd .
    qemu-system-x86_64 -L OVMF_dir/ -pflash OVMF_CODE.4m.fd -cdrom cdimage.iso
fi

if [[ "$1" == "--run" && "$1" != "--iso" ]]; then
    qemu-system-x86_64 -enable-kvm \
        -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.4m.fd \
        -drive if=pflash,format=raw,readonly=on,file=OVMF_VARS.4m.fd \
        -drive format=raw,file=fat:rw:esp
fi