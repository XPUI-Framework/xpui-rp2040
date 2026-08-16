/* The RP2040's flash map, and the RAM the linker may hand out.
 *
 * BOOT2 is the first 256 bytes of flash: the second-stage bootloader the boot
 * ROM copies into RAM and runs before anything else. embassy-rp supplies the
 * blob and its own `link-rp.x` places it here.
 *
 * FLASH is declared as the 2 MB a Badger 2040 has. A Tufty 2040 has 8 MB;
 * under-declaring costs only the space, and one file that is right for both
 * boards is worth more than two that differ by a constant.
 *
 * RAM is the 256 kB striped bank. The RP2040 has 264 kB in total, but the two
 * 4 kB scratch banks above it are not contiguous with the rest, so — as in
 * every RP2040 linker script — they are left out.
 */
MEMORY {
    BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100
    FLASH : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100
    RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}
