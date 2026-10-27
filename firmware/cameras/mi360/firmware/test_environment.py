"""Isolated activation-verifier tests; never reads or writes a device."""
import struct
import unittest
import zlib
import verify_environment as check


def boot(flag=b'1', **changes):
    variables = {b'boot_flag': flag, b'bootcmd': check.COMMANDS[flag], b'boot_retry': b'0', b'unchanged': b'preserve'}
    variables.update({key.encode(): value for key, value in changes.items()})
    payload = b'\0'.join(key + b'=' + value for key, value in sorted(variables.items())) + b'\0\0'
    payload = payload.ljust(check.ENV_SIZE - 4, b'\xff')
    image = bytearray(b'\xff' * check.BOOT_SIZE)
    image[check.ENV_START:] = struct.pack('<I', zlib.crc32(payload)) + payload
    return bytes(image)


class Environment(unittest.TestCase):
    def test_only_the_intended_transition_is_accepted(self):
        check.verify(boot(), boot(b'0'), 'A')
        check.verify(boot(b'0'), boot(), 'B')

    def test_crc_protected_bytes_other_variables_and_wrong_slot_are_refused(self):
        before, after = boot(), boot(b'0')
        corrupt = bytearray(after)
        corrupt[-1] ^= 1
        outside = bytearray(after)
        outside[100] ^= 1
        for value in [bytes(corrupt), bytes(outside), boot(b'0', unchanged=b'changed'), before, after[:-1]]:
            with self.subTest(value_length=len(value)), self.assertRaises(ValueError):
                check.verify(before, value, 'A')


if __name__ == '__main__':
    unittest.main()
