#!/usr/bin/env python3
"""Print a hash of a Mach-O file's content without its code signature.

A code signature (ad hoc or Developer ID) depends on the signing tool's version, the certificate and the time, so two
correct builds of the same source never have identical signed files. This hashes what the signature covers instead:
the file up to where the signature starts, with the fields that only record the signature's size set to zero
(LC_CODE_SIGNATURE's offset and size, and __LINKEDIT's file and memory size). Universal (fat) files are handled per
architecture. Files that aren't Mach-O are hashed as they are.

The build UUID (LC_UUID) is zeroed too. Apple's linker documents it as a hash of the output's content, but it doesn't
always come out the same: two CI builds of ffmpeg were identical in every byte of code, data and symbols, and differed
only in the UUID (and so in the signature, which covers it). Every other byte still has to match.
"""
import hashlib, struct, sys

MH_MAGIC_64, FAT_MAGIC, FAT_MAGIC_64 = 0xFEEDFACF, 0xCAFEBABE, 0xCAFEBABF
LC_SEGMENT_64, LC_UUID, LC_CODE_SIGNATURE = 0x19, 0x1B, 0x1D


def thin(b: bytes) -> bytes:
    if struct.unpack_from("<I", b, 0)[0] != MH_MAGIC_64:
        raise ValueError("not a 64-bit little-endian Mach-O")
    ncmds = struct.unpack_from("<I", b, 16)[0]
    out, off, end = bytearray(b), 32, len(b)
    for _ in range(ncmds):
        cmd, size = struct.unpack_from("<II", b, off)
        if cmd == LC_CODE_SIGNATURE:
            end = struct.unpack_from("<I", b, off + 8)[0]
            struct.pack_into("<II", out, off + 8, 0, 0)
        elif cmd == LC_UUID:
            out[off + 8:off + 24] = bytes(16)
        elif cmd == LC_SEGMENT_64 and b[off + 8:off + 24].rstrip(b"\0") == b"__LINKEDIT":
            struct.pack_into("<Q", out, off + 32, 0)  # vmsize
            struct.pack_into("<Q", out, off + 48, 0)  # filesize
        off += size
    return bytes(out[:end])


def normalise(b: bytes) -> bytes:
    magic = struct.unpack_from(">I", b, 0)[0] if len(b) >= 4 else 0
    if magic in (FAT_MAGIC, FAT_MAGIC_64):
        n = struct.unpack_from(">I", b, 4)[0]
        parts = []
        for i in range(n):
            if magic == FAT_MAGIC:
                cpu, sub, o, s, _ = struct.unpack_from(">5I", b, 8 + 20 * i)
            else:
                cpu, sub, o, s, _, _ = struct.unpack_from(">IIQQII", b, 8 + 32 * i)
            parts.append(struct.pack(">II", cpu, sub) + thin(b[o:o + s]))
        return b"".join(parts)
    if len(b) >= 4 and struct.unpack_from("<I", b, 0)[0] == MH_MAGIC_64:
        return thin(b)
    return b


for path in sys.argv[1:]:
    with open(path, "rb") as f:
        print(hashlib.sha256(normalise(f.read())).hexdigest(), path)
