#!/usr/bin/env python3
"""Read final PAK bytes and independently decompress its filesystem with dissect."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import struct
import zlib

from dissect.squashfs import SquashFS

from ubi import STOCK_SHA256, parse_ubi
from build import WORKER_SHA256

BASE = Path(__file__).resolve().parent


def digest(data):
    return hashlib.sha256(data).hexdigest()


def inventory(node, path='/'):
    entry = {'mode': node.mode, 'uid': node.uid, 'gid': node.gid,
             'mtime': node.mtime.isoformat()}
    if node.is_file():
        with node.open() as handle:
            data = handle.read()
        entry.update(bytes=len(data), sha256=digest(data))
    elif node.is_symlink():
        entry['target'] = node.link
    elif not node.is_dir():
        raise ValueError(f'Unhandled inode type {path}')
    yield path, entry
    if node.is_dir():
        for child in node.iterdir():
            yield from inventory(child, path.rstrip('/') + '/' + child.name)


def sections(data):
    assert len(data) >= 0x728
    assert struct.unpack_from('<I', data)[0] == 0x32725913
    assert struct.unpack_from('<I', data, 8)[0] == 0x4102
    crc = zlib.crc32(data[0x728:], 0xffffffff)
    crc = zlib.crc32(struct.pack('<I', 2), crc)
    crc = zlib.crc32(data[12:0x34c], crc) ^ 0xffffffff
    assert crc == struct.unpack_from('<I', data, 4)[0], 'Final PAK CRC differs'
    end = 0x728
    result = {}
    for index in range(13):
        name, version, start, size = struct.unpack_from('<32s24sII', data, 12 + 64 * index)
        name = name.rstrip(b'\0')
        if not name:
            continue
        assert name not in result and start == end and size > 0
        assert start + size <= len(data)
        result[name] = (version, data[start:start + size])
        end = start + size
    assert end == len(data)
    assert set(result) == {b'loader', b'fdt', b'uboot', b'kernel', b'rootfs', b'app'}
    return result


def verify(stock_path, directory):
    manifest = json.loads((directory / 'manifest.json').read_text())
    stock_data = stock_path.read_bytes()
    assert digest(stock_data) == STOCK_SHA256, 'Wrong original stock PAK'
    candidate_path = Path(manifest['candidate']['path']).resolve()
    assert candidate_path.parent == directory.resolve(), 'Candidate lies outside build directory'
    candidate_data = candidate_path.read_bytes()
    assert digest(candidate_data) == manifest['candidate']['sha256'], 'Final PAK SHA-256 differs'
    before_pak, after_pak = sections(stock_data), sections(candidate_data)
    assert stock_data[0x34c:0x728] == candidate_data[0x34c:0x728], 'MTD table changed'
    assert stock_data[8:12] == candidate_data[8:12]
    for name, (version, payload) in before_pak.items():
        assert after_pak[name][0] == version, 'Section version changed'
        if name != b'rootfs':
            assert after_pak[name][1] == payload, 'Preserved section changed'
    candidate_ubi = after_pak[b'rootfs'][1]
    assert (directory / 'rootfs.ubi').read_bytes() == candidate_ubi, 'Extract the actual final PAK rootfs'
    stock = parse_ubi(before_pak[b'rootfs'][1])['payload']
    assert (directory / 'stock.squashfs').read_bytes() == stock
    independent = list((directory / 'independent-ubi-extract').rglob('*.ubifs'))
    assert len(independent) == 1, 'Run ubireader_extract_images into a fresh directory'
    extracted = independent[0].read_bytes()
    candidate = (directory / 'rootfs.squashfs').read_bytes()
    assert parse_ubi(candidate_ubi)['payload'] == candidate[:struct.unpack_from('<Q', candidate, 40)[0]]
    assert extracted[:len(candidate)] == candidate
    assert all(value == 255 for value in extracted[len(candidate):])
    before = dict(inventory(SquashFS(io.BytesIO(stock)).get('/')))
    after = dict(inventory(SquashFS(io.BytesIO(extracted)).get('/')))
    added = {'/root/.ssh': 0o40700, '/root/.ssh/authorized_keys': 0o100600,
             '/usr/sbin/dropbear': 0o100755, '/usr/bin/dropbearkey': 0o120777,
             '/etc/init.d/S99_zzRootSSH': 0o100755}
    assert set(after) - set(before) == set(added)
    assert not set(before) - set(after)
    for path, entry in before.items():
        expected = dict(entry)
        if path == '/root':
            expected.update(mode=0o40700, uid=0, gid=0)
        assert after[path] == expected, path
    for path, mode in added.items():
        assert after[path]['uid'] == after[path]['gid'] == 0
        assert after[path]['mode'] == mode, path
    assert after['/usr/bin/dropbearkey']['target'] == '../sbin/dropbear'
    assert after['/usr/sbin/dropbear']['sha256'] == manifest['ssh_binary_sha256'] == WORKER_SHA256
    assert after['/root/.ssh/authorized_keys']['sha256'] == manifest['authorized_key_sha256']
    assert after['/etc/init.d/S99_zzRootSSH']['sha256'] == digest((BASE / 'S99_zzRootSSH').read_bytes())
    return {'passed': True, 'reader': 'dissect.squashfs 1.12',
        'ubi_extractor': 'ubi-reader 0.8.16', 'final_pak_hash_crc_sections_and_mtd_checked': True,
        'stock_entries_verified': len(before), 'new_paths': sorted(added),
        'only_existing_path_change': '/root mode0700 uid0 gid0',
        'all_stock_regular_file_bytes_read_and_compared': True,
        'symlinks_modes_owners_timestamps_compared': True,
        'independently_extracted_payload_matches': True,
        'ssh_binary_sha256': WORKER_SHA256,
        'limits': 'Offline content verification; no hardware boot, acceptance or recovery proof'}


def main():
    if not __debug__:
        raise SystemExit('Run without -O: integrity checks must remain enabled')
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('stock_pak', type=Path)
    parser.add_argument('build_dir', type=Path)
    args = parser.parse_args()
    directory = args.build_dir.resolve()
    result = verify(args.stock_pak, directory)
    (directory / 'independent-filesystem-validation.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
