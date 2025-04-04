set -e
cargo build --target x86_64-unknown-uefi
mkdir -p esp/efi/boot
cp target/x86_64-unknown-uefi/debug/first-uefi-rust.efi esp/efi/boot/bootx64.efi
cp /usr/share/edk2/x64/OVMF_CODE.4m.fd .
cp /usr/share/edk2/x64/OVMF_VARS.4m.fd .

qemu-system-x86_64 -enable-kvm -nographic \
    -drive if=pflash,format=raw,readonly=on,file=OVMF_CODE.4m.fd \
    -drive if=pflash,format=raw,readonly=on,file=OVMF_VARS.4m.fd \
    -drive format=raw,file=fat:rw:esp