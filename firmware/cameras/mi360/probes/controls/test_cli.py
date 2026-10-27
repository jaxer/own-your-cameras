#!/usr/bin/env python3
"""Host CLI refusals; no device paths on the host are opened by these cases."""
import pathlib
import subprocess
import tempfile

BASE = pathlib.Path(__file__).resolve().parent
with tempfile.TemporaryDirectory(prefix='mi360-cli-') as tmp:
    exe = pathlib.Path(tmp) / 'camctl'
    subprocess.run(['cc', '-std=c11', '-Wall', '-Wextra', '-Werror', '-fsanitize=address,undefined',
                    str(BASE/'camctl.c'), str(BASE/'motor.c'), '-o', str(exe)], check=True)
    help_result = subprocess.run([exe, '--help'], capture_output=True, text=True, check=True)
    assert 'motor-roundtrip' in help_result.stdout
    for args in [[], ['motor-jog','999999999999999999999','0'], ['motor-jog','1','0','901'],
                 ['motor-jog','0','0','bad'], ['led-test','red'], ['led-test','blue','1001'],
                 ['ircut','garbage'], ['motor-jog',' 1','0']]:
        r = subprocess.run([exe, *args], capture_output=True, text=True)
        assert r.returncode == 2, (args, r.returncode, r.stderr)
print('CLI argument refusal checks passed')
