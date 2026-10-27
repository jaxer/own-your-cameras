#!/usr/bin/env python3 -I
"""Send an existing real recording to cam's PCMU backchannel; never open a microphone.

Convert a real recording first: ffmpeg -i speech.wav -ar 8000 -ac 1 -f mulaw speech.pcmu
The camera must already be running cam serve. This tool changes nothing else.
"""
import argparse
import hashlib
from pathlib import Path
import re
import socket
import struct
import time


def send_pcmu(host, port, path, data, credentials=None):
    """Pace existing 8 kHz mono PCMU at 160 bytes/20 ms. Return packet count."""
    if not data:
        raise ValueError('recording is empty')
    url = f'rtsp://{host}:{port}/{path}'
    with socket.create_connection((host, port), timeout=5) as sock, sock.makefile('rb') as stream:
        sock.settimeout(10)  # Camera DESCRIBE may wait five seconds for its first video keyframe.
        sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        cseq, digest = 0, None

        def call(method, target, extra=''):
            nonlocal cseq, digest
            for attempt in range(2):
                cseq += 1
                auth = ''
                if digest and credentials:
                    realm, nonce = digest
                    user, password = credentials
                    md5 = lambda value: hashlib.md5(value.encode()).hexdigest()
                    response = md5(f'{md5(f"{user}:{realm}:{password}")}:{nonce}:{md5(f"{method}:{target}")}')
                    auth = f'Authorization: Digest username="{user}", realm="{realm}", nonce="{nonce}", uri="{target}", response="{response}"\r\n'
                sock.sendall(f'{method} {target} RTSP/1.0\r\nCSeq: {cseq}\r\n{auth}{extra}\r\n'.encode())
                # Bound the whole response before allocating a body or buffering headers.
                limit = 16 * 1024
                used = 0

                def line():
                    nonlocal used
                    value = stream.readline(limit - used + 1)
                    used += len(value)
                    if used > limit or not value.endswith(b'\r\n'):
                        raise ValueError('invalid or oversized RTSP response headers')
                    return value

                status = line().decode().strip()
                headers = {}
                while (value := line()) != b'\r\n':
                    key, value = value.decode().split(':', 1)
                    key = key.lower()
                    if key == 'content-length' and key in headers:
                        raise ValueError('duplicate RTSP response content length')
                    headers[key] = value.strip()
                length = headers.get('content-length', '0')
                if not length.isascii() or not length.isdecimal() or len(length) > 5:
                    raise ValueError('invalid RTSP response content length')
                length = int(length)
                if used + length > limit:
                    raise ValueError('oversized RTSP response body')
                if len(stream.read(length)) != length:
                    raise EOFError('camera closed during RTSP response body')
                if ' 200 ' in status:
                    return headers
                if ' 401 ' in status and credentials and attempt == 0:
                    fields = dict(re.findall(r'(\w+)="([^"]*)"', headers.get('www-authenticate', '')))
                    digest = fields['realm'], fields['nonce']
                    continue
                raise RuntimeError(f'{method}: {status}')
            raise RuntimeError('authentication failed')

        call('DESCRIBE', url, 'Require: www.onvif.org/ver20/backchannel\r\nAccept: application/sdp\r\n')
        headers = call('SETUP', url + '/trackID=2', 'Transport: RTP/AVP/TCP;unicast;interleaved=4-5\r\n')
        session = headers['session'].split(';')[0]
        call('PLAY', url, f'Session: {session}\r\n')
        start = time.monotonic()
        packets = 0
        for offset in range(0, len(data), 160):
            time.sleep(max(0, start + offset / 8000 - time.monotonic()))
            payload = data[offset:offset + 160]
            rtp = struct.pack('!BBHII', 0x80, 0, packets & 65535, offset & 0xffffffff, 31) + payload
            sock.sendall(b'$\x04' + struct.pack('!H', len(rtp)) + rtp)
            packets += 1
        # Let the last packet reach the hardware loop before releasing the speaker lease.
        time.sleep(0.05)
        cseq += 1
        sock.sendall(f'TEARDOWN {url} RTSP/1.0\r\nCSeq: {cseq}\r\nSession: {session}\r\n\r\n'.encode())
        return packets


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host', required=True, help="the camera's address")
    parser.add_argument('--port', type=int, default=554)
    parser.add_argument('--path', choices=['Preview_01_main', 'Preview_01_sub'], default='Preview_01_sub')
    parser.add_argument('--auth', type=Path, help='file containing existing user:password')
    parser.add_argument('pcmu', type=Path, help='8 kHz mono PCMU from an existing real recording')
    args = parser.parse_args()
    credentials = tuple(args.auth.read_text().strip().split(':', 1)) if args.auth else None
    data = args.pcmu.read_bytes()
    packets = send_pcmu(args.host, args.port, args.path, data, credentials)
    print(f'pcmu_packets={packets} pcmu_bytes={len(data)} seconds={len(data) / 8000:.3f}')


if __name__ == '__main__':
    main()
