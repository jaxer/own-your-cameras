#!/usr/bin/env python3
"""Describe a firmware upload offline; --flash performs one upload with no retries."""
import argparse
import getpass
import hashlib
import json
from pathlib import Path
import ssl
import urllib.error
import urllib.parse
import urllib.request
import uuid

from build import pak_sections
from ubi import require
import struct
import zlib

EXPECTED = {'model': 'E1 Zoom', 'hardVer': 'IPC_566SD664M5MP',
            'firmVer': 'v3.1.0.4417_2412122178'}
CHUNK_BYTES = 40960


class Session:
    def __init__(self, host, username, password, insecure=False):
        # A host only: never accept embedded credentials, paths or query strings.
        require(bool(host) and not any(c in host for c in '/@?#:'), 'Provide a hostname only')
        self.host = host
        self.token = None
        context = ssl._create_unverified_context() if insecure else ssl.create_default_context()
        self.opener = urllib.request.build_opener(urllib.request.ProxyHandler({}),
            urllib.request.HTTPSHandler(context=context))
        login = self.call('Login', {'User': {'userName': username, 'password': password, 'Version': '0'}})
        self.token = login['value']['Token']['name']

    def request(self, command, body, content_type, extra=None):
        query = {'cmd': command}
        if self.token:
            query['token'] = self.token
        query.update(extra or {})
        url = f'https://{self.host}/cgi-bin/api.cgi?' + urllib.parse.urlencode(query)
        request = urllib.request.Request(url, data=body, headers={'Content-Type': content_type})
        try:
            with self.opener.open(request, timeout=120) as response:
                records = json.load(response)
        except Exception as error:
            # HTTP/transport exceptions can contain tokens in URLs. Never echo them.
            raise RuntimeError(f'{command}: {type(error).__name__}; stopped without retry') from None
        require(isinstance(records, list) and len(records) == 1, 'Unexpected response shape')
        record = records[0]
        require(record.get('cmd') == command, 'Unexpected response command')
        require(record.get('code') == 0, f'{command}: API rejected request')
        require(record.get('value', {}).get('rspCode', 200) == 200, f'{command}: API failure')
        return record

    def call(self, command, param=None):
        body = json.dumps([{'cmd': command, 'action': 0, 'param': param or {}}]).encode()
        return self.request(command, body, 'application/json')


def checked_image(path, expected_sha):
    data = path.read_bytes()
    require(hashlib.sha256(data).hexdigest() == expected_sha, 'Image SHA-256 differs from reviewed hash')
    require(len(data) >= 0x728 and struct.unpack_from('<I', data)[0] == 0x32725913, 'Wrong PAK magic')
    require(struct.unpack_from('<I', data, 8)[0] == 0x4102, 'Wrong PAK format word')
    pak_sections(data)
    crc = zlib.crc32(data[0x728:], 0xffffffff)
    crc = zlib.crc32(struct.pack('<I', 2), crc)
    crc = zlib.crc32(data[12:0x34c], crc) ^ 0xffffffff
    require(crc == struct.unpack_from('<I', data, 4)[0], 'PAK CRC mismatch')
    require(all(c.isalnum() or c in '._-' for c in path.name) and path.name.endswith('.pak'),
            'Expected a plain PAK basename')
    return data


def upload(session, path, data):
    identity = session.call('GetDevInfo')['value']['DevInfo']
    require(all(identity.get(k) == v for k, v in EXPECTED.items()), 'Unsupported device/build; stopped')
    session.call('UpgradePrepare', {'restoreCfg': 0, 'fileName': path.name})
    upload_id = str(uuid.uuid4())
    boundary = '----ReolinkFirmwareChunk'
    count = (len(data) + CHUNK_BYTES - 1) // CHUNK_BYTES
    for index in range(count):
        chunk = data[index * CHUNK_BYTES:(index + 1) * CHUNK_BYTES]
        filename = f'{path.name}&{upload_id}&{index}&{len(data)}'
        body = (f'--{boundary}\r\nContent-Disposition: form-data; name="upgrade-package"; '
                f'filename="{filename}"\r\nContent-Type: application/octet-stream\r\n\r\n').encode()
        body += chunk + f'\r\n--{boundary}--\r\n'.encode()
        session.request('Upgrade', body, f'multipart/form-data; boundary={boundary}',
                        {'file': 'upgrade-package', 'clearConfig': 0})
        if (index + 1) % 40 == 0 or index + 1 == count:
            print(f'Acknowledged {index + 1}/{count} chunks', flush=True)
    # The updater owns installation/reboot. No separate reboot or logout traffic.
    print('Upload acknowledged. Wait for installation/reboot; verify SSH and settings separately.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('candidate', type=Path)
    parser.add_argument('--sha256', required=True, help='Hash of the exact reviewed local image')
    parser.add_argument('--host', help='CAMERA_HOST; required only with --flash')
    parser.add_argument('--username', default='admin')
    parser.add_argument('--insecure', action='store_true', help='Explicitly allow a self-signed HTTPS certificate')
    parser.add_argument('--flash', action='store_true', help='Upload once; device will install and reboot')
    args = parser.parse_args()
    data = checked_image(args.candidate, args.sha256)
    print(json.dumps({'bytes': len(data), 'sha256': args.sha256,
        'chunks': (len(data) + CHUNK_BYTES - 1) // CHUNK_BYTES,
        'restoreCfg': 0, 'clearConfig': 0, 'expected_device': EXPECTED}, indent=2))
    if not args.flash:
        return
    require(bool(args.host), '--host is required for flashing')
    password = getpass.getpass('Camera administrator password: ')
    session = Session(args.host, args.username, password, args.insecure)
    upload(session, args.candidate, data)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, RuntimeError) as error:
        raise SystemExit(str(error)) from None
