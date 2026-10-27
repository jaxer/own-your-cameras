"""Offline format and preservation checks for MJSXJ10CM firmware 4.3.4_0376."""
import base64
import hashlib
import re
import struct
import zlib

FLASH_SIZE = 0x1000000
ENV_START, ENV_SIZE = 0x48000, 0x8000
USR_START, USR_SIZE = 0xaa0000, 0x470000
KERNEL_START, KERNEL_SIZE = 0x8f0000, 0x1b0000
TIMESTAMP = 1792972800  # Fixed recipe timestamp, matching the original image.
LAYOUT = [('BOOT', 0x50000), ('ROOTFS', 0x280000), ('A_KERNEL', 0x1b0000),
          ('A_USRFS', 0x470000), ('B_KERNEL', 0x1b0000), ('B_USRFS', 0x470000),
          ('DATA', 0x40000), ('SOUND', 0xa0000), ('FACTORY', 0x10000)]
MARKER = b'# START OF DATA - DO NOT MODIFY\n#\n'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def check_kernel(data):
    if len(data) < 64 or len(data) > KERNEL_SIZE:
        raise ValueError('Kernel size is outside this layout')
    fields = struct.unpack_from('>7I4B32s', data)
    header = bytearray(data[:64])
    header[4:8] = bytes(4)
    size = fields[3]
    if (fields[0] != 0x27051956 or not size or size > len(data) - 64
            or fields[7:11] != (5, 5, 2, 3)
            or zlib.crc32(header) != fields[1]
            or zlib.crc32(data[64:64 + size]) != fields[6]
            or any(byte != 255 for byte in data[64 + size:])):
        raise ValueError('Invalid MIPS Linux LZMA uImage or CRC/padding')


def check_squashfs(data):
    if len(data) < 96 or data[:4] != b'hsqs':
        raise ValueError('Missing SquashFS superblock')
    block_size = struct.unpack_from('<I', data, 12)[0]
    compression = struct.unpack_from('<H', data, 20)[0]
    major, minor = struct.unpack_from('<HH', data, 28)
    used = struct.unpack_from('<Q', data, 40)[0]
    if (block_size != 131072 or compression != 4 or (major, minor) != (4, 0)
            or not 96 <= used <= len(data)):
        raise ValueError('Expected bounded SquashFS 4.0/XZ, 128 KiB blocks')
    if struct.unpack_from('<Q', data, 56)[0] != 0xffffffffffffffff:
        raise ValueError('Extended attributes are outside this preservation recipe')
    return used


def environment(data):
    if len(data) < ENV_START + ENV_SIZE:
        raise ValueError('Truncated boot environment')
    payload = bytes(data[ENV_START + 4:ENV_START + ENV_SIZE])
    if zlib.crc32(payload) != struct.unpack_from('<I', data, ENV_START)[0]:
        raise ValueError('Stored U-Boot environment CRC mismatch')
    variables = payload.split(b'\0\0', 1)
    if len(variables) != 2:
        raise ValueError('Unterminated boot environment')
    env = {}
    for record in variables[0].split(b'\0'):
        name, separator, value = record.partition(b'=')
        if not separator or name in env:
            raise ValueError('Invalid or duplicate boot environment variable')
        env[name] = value
    return env


def check_dump(data, expected_sha256):
    if not re.fullmatch('[0-9a-f]{64}', expected_sha256):
        raise ValueError('Supply the lowercase SHA-256 recorded for your original dump')
    if len(data) != FLASH_SIZE or digest(data) != expected_sha256:
        raise ValueError('Original dump size or expected SHA-256 does not match')
    env = environment(data)
    match = re.search(rb'(?:^| )mtdparts=jz_sfc:([^ ]+)(?: |$)', env.get(b'bootargs', b''))
    specification = ','.join(f'{size // 1024}k({name})' for name, size in LAYOUT).encode()
    if match is None or match[1] != specification:
        raise ValueError('Partition map differs from the inspected MJSXJ10CM layout')
    if (env.get(b'boot_flag') != b'1' or env.get(b'imi_version') != b'4.3.4_0376'
            or env.get(b'bootcmd') != b'sf probe;sf read 0x80600000 0x8f0000 0x1b0000;bootm 0x80600000'):
        raise ValueError('Recipe requires selected B firmware 4.3.4_0376 and its original boot command')
    offset = 0
    for name, size in LAYOUT:
        part = data[offset:offset + size]
        if name.endswith('KERNEL'):
            check_kernel(part)
        elif name in ('ROOTFS', 'A_USRFS', 'B_USRFS', 'SOUND'):
            check_squashfs(part)
        offset += size


def public_key(data):
    lines = data.strip().splitlines()
    fields = lines[0].split() if len(lines) == 1 else []
    if len(fields) < 2 or fields[0] != 'ssh-ed25519':
        raise ValueError('Expected one Ed25519 public key, without authorized_keys options')
    decoded = base64.b64decode(fields[1], validate=True)
    if len(decoded) != 51 or decoded[:19] != b'\0\0\0\x0bssh-ed25519\0\0\0\x20':
        raise ValueError('Malformed Ed25519 public key')
    return ' '.join(fields[:2]) + '\n'  # Discard the owner-selected key comment.


def check_static_mips(data):
    if (len(data) < 52 or data[:7] != b'\x7fELF\x01\x01\x01'
            or struct.unpack_from('<HH', data, 16) != (2, 8)
            or struct.unpack_from('<I', data, 36)[0] & 0xf000f000 != 0x70001000):
        raise ValueError('Expected a static MIPS32r2 little-endian O32 executable')
    offset = struct.unpack_from('<I', data, 28)[0]
    size, count = struct.unpack_from('<HH', data, 42)
    if not count or size != 32 or offset < 52 or offset + size * count > len(data):
        raise ValueError('Invalid ELF program header table')
    if any(struct.unpack_from('<I', data, offset + i * size)[0] in (2, 3) for i in range(count)):
        raise ValueError('ELF has dynamic dependencies or an interpreter')


def pseudo_manifest(blob):
    """Compare file bytes and all metadata emitted by unsquashfs -pf."""
    header, payload = blob.split(MARKER, 1)
    result = {}
    for line in header.decode().splitlines():
        if not line or line.startswith('#'):
            continue
        fields = line.split()
        if len(fields) < 6:
            raise ValueError('Unsupported pseudo-file entry')
        name, kind = fields[:2]
        if name in result:
            raise ValueError('Duplicate pseudo-file path')
        item = {'type': kind, 'mtime': int(fields[2]), 'mode': fields[3],
                'uid': int(fields[4]), 'gid': int(fields[5])}
        if kind == 'R':
            size, offset, sparse = map(int, fields[6:9])
            if sparse or size < 0 or offset < 0 or offset + size > len(payload):
                raise ValueError('Unsupported sparse or truncated pseudo-file data')
            item.update(size=size, sha256=digest(payload[offset:offset + size]))
        elif kind == 'S':
            item['target'] = ' '.join(fields[6:])
        elif kind in ('D', 'C', 'B', 'I', 'L'):
            item['extra'] = fields[6:]
        else:
            raise ValueError('Unknown pseudo-file type')
        result[name] = item
    return result


def patch_dump(stock, application):
    check_squashfs(application)
    if len(stock) != FLASH_SIZE or len(application) > USR_SIZE:
        raise ValueError('Dump size mismatch or B_USRFS overflow')
    result = stock[:USR_START] + application.ljust(USR_SIZE, b'\xff') + stock[USR_START + USR_SIZE:]
    check_preserved(stock, result)
    return result


def check_preserved(stock, candidate):
    if (len(stock) != FLASH_SIZE or len(candidate) != FLASH_SIZE
            or stock[:USR_START] != candidate[:USR_START]
            or stock[USR_START + USR_SIZE:] != candidate[USR_START + USR_SIZE:]):
        raise ValueError('Bytes outside B_USRFS changed, or full-chip size differs')
