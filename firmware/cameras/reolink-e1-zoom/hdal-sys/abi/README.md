# ABI reading tools

Helpers for reading `libhdal.so`'s parameter dispatch from its disassembly. What they are for, and why their
output is only a list of candidates: [../../docs/hdal-abi.md](../../docs/hdal-abi.md#the-method).

| File | What it is |
|---|---|
| `libhdal-2.00-param-sizes.tsv` | the parameters the daemon calls, with the byte counts `tests/abi.rs` holds the headers to |
| `draft_table.py` | drafts an unreviewed inventory: every declared get/set id with its header size and the binary's jump-table and copy-length candidates in separate columns |
| `disasm_cases.py` | prints the jump-table cases of one function |
| `show.py` | prints a half-open address range of the disassembly |
| `audio-config-fault.c` | a RAM-only fault experiment for the audio configuration path; never part of the product |

## Running them

These optional maintainer tools need a separately retained, hash-verified original library,
its reference headers and an ARM disassembler. They are not part of normal builds.

```sh
mkdir -p target/abi
arm-none-eabi-objdump -d /path/to/original/libhdal.so > target/abi/hdal.asm
python3 -I hdal-sys/abi/draft_table.py target/abi/hdal.asm hdal-sys/src/header_sizes.rs \
  /path/to/reference/headers > target/abi/candidates.tsv
python3 -I hdal-sys/abi/disasm_cases.py target/abi/hdal.asm hd_videocap_set
python3 -I hdal-sys/abi/show.py target/abi/hdal.asm 1c518 1c550
```

Never redirect the draft into `libhdal-2.00-param-sizes.tsv`; the draft has ten columns, the table eight:

| Column | Meaning |
|---|---|
| unit, op, id, name | the header declaration, once for get and once for set |
| struct, header_bytes | the header comment's suggested type and clang's size, or unknown |
| status | always `unreviewed` |
| dispatch_candidates | `switch:index_register->target`; index + bias is only a candidate id |
| copy_candidates | bounded instruction-scan observations, independent of the header size |
| legacy_pin | the table's claim, to recheck, never an override |

`disasm_cases.py` recognises one ARM jump-table pattern and walks up to 90 instructions per case. It does not trace
every path or callee, every register alias or pointer origin; a call or an unmodelled `r2` use drops a tracked count.
The enum reader only accepts the implicit one-entry-per-line form and fails on anything else rather than renumber.
Record the library hash, disassembler version and source revision with any reading.

Host tests: `python3 -I -m unittest discover -s scripts -p test_hdal_abi_tools.py` (part of `make check`).
