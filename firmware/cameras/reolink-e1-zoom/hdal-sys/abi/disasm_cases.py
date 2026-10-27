#!/usr/bin/env python3 -I
"""List heuristic switch/copy candidates in GNU ARM objdump output, not ABI sizes.

The bounded walk does not prove reachability, switch identity, pointer provenance,
operation direction or complete user-buffer extents. It does not follow callees or
all conditional paths. STM totals can span unrelated destinations. Missing counts
are unknown, never proof of a word access or an unsupported parameter.

usage: disasm_cases.py libhdal.asm hd_videocap_get [...]
"""
import re
import sys

INSN = re.compile(r"^\s*([0-9a-f]+):\t[0-9a-f]+\s+\t?(\S+)\s*(.*)$")
FUNC = re.compile(r"^([0-9a-f]+) <(\S+?)(?:@@Base)?>:$")


def load(path):
    funcs, cur = {}, None
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.rstrip("\n")
        m = FUNC.match(line)
        if m:
            cur = m.group(2)
            funcs[cur] = []
            continue
        m = INSN.match(line)
        if m and cur:
            # '@' inside <memcpy@plt> is part of the symbol, not an ARM comment.
            ops = re.split(r"\s+@", m.group(3), maxsplit=1)[0].strip()
            funcs[cur].append((int(m.group(1), 16), m.group(2), ops))
    return funcs


def imm(ops):
    m = re.search(r"#(-?\d+)", ops)
    return int(m.group(1)) if m else None


def tables(insns):
    """Yield (index_register, bias, [targets]) for every `addls pc, pc, rN, lsl` by 2 (a jump table)."""
    by_addr = {a: i for i, (a, _, _) in enumerate(insns)}
    for i, (addr, op, ops) in enumerate(insns):
        if op == "addls" and ops.startswith("pc, pc, "):
            reg = ops.split(",")[2].strip()
            bias, cmp_k = 0, None
            for j in range(i - 1, max(i - 8, -1), -1):
                _, pop, pops = insns[j]
                if pop == "cmp" and pops.startswith(reg + ","):
                    cmp_k = imm(pops)
                    break
            # a `sub reg, src, #K` just before the cmp shifts the id
            for j in range(i - 1, max(i - 12, -1), -1):
                _, pop, pops = insns[j]
                m = re.match(rf"sub\s*{reg}, (\w+), #(\d+)", pop + " " + pops)
                if m:
                    bias = int(m.group(2))
                    break
            targets = []
            k = i + 2  # first table slot is at pc+8 → two instructions on
            n = (cmp_k or 0) + 1
            for t in range(n):
                if k + t >= len(insns):
                    break
                _, bop, bops = insns[k + t]
                if bop != "b":
                    break
                targets.append(int(bops.split()[0], 16))
            yield addr, reg, bias, cmp_k, targets


def case_sizes(insns, by_addr, start, stop_at):
    sizes, i, steps, r2 = [], by_addr.get(start), 0, None
    while i is not None and i < len(insns) and steps < 90:
        addr, op, ops = insns[i]
        if steps and addr in stop_at:
            break
        if op in ("mov", "movw") and ops.startswith("r2,"):
            r2 = imm(ops)
        elif re.search(r"\br2\b", ops):
            # Conservative: even an unmodelled use may write r2 (e.g. LDM).
            r2 = None
        if op == "bl":
            kind = re.search(r"<(memcpy|memset|memmove)(?:@[^>]*)?>", ops)
            if kind and r2 is not None:
                sizes.append((kind.group(1), r2))
        if op.startswith("b"):
            # AAPCS calls clobber r2; conditional branches invalidate this linear guess.
            r2 = None
        if op.startswith("stm") and not ops.startswith("sp"):
            regs = ops.split("{")[1].split("}")[0].split(",")
            n = sum(1 for r in regs if "-" not in r) + sum(
                int(r.split("-")[1][1:]) - int(r.split("-")[0].strip()[1:]) + 1 for r in regs if "-" in r
            )
            sizes.append(("stm", 4 * n))
        if op == "b":
            tgt = int(ops.split()[0], 16)
            if tgt in stop_at or tgt not in by_addr or tgt < addr:
                break
            i = by_addr[tgt]
        elif op in ("pop", "bx") or (op == "ldm" and "pc" in ops):
            break
        else:
            i += 1
        steps += 1
    stm = sum(n for k, n in sizes if k == "stm")
    out = [s for s in sizes if s[0] != "stm"]
    return out + ([("stm-total", stm)] if stm else [])


def main():
    funcs = load(sys.argv[1])
    for name in sys.argv[2:]:
        insns = funcs[name]
        by_addr = {a: i for i, (a, _, _) in enumerate(insns)}
        tabs = list(tables(insns))
        stop = {t for _, _, _, _, ts in tabs for t in ts}
        for addr, reg, bias, cmp_k, targets in tabs:
            print(f"{name} switch@{addr:x} index={reg} bias={bias} cmp={cmp_k} slots={len(targets)}")
            for idx, t in enumerate(targets):
                sz = case_sizes(insns, by_addr, t, stop - {t})
                print(f"  idx {idx:2d} (id {idx + bias:2d}) -> {t:x}  {sz}")


if __name__ == "__main__":
    main()
