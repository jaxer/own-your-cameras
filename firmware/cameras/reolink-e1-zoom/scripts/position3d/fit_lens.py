"""Local steps-per-frame-width across the frame by large-patch template matching (blur tolerant).

usage: fit_lens.py pan|tilt FILE...  (frames from shoot.sh, named p<pan>_t<tilt>_z<zoom>.jpg)
Prints a, b of s'(x) = a + 3 b x^2 (so s(x) = a x + b x^3) and the raw samples.
"""
import itertools
import re
import sys
import cv2
import numpy as np


def pos(name):
    return tuple(int(v) for v in re.search(r"p(\d+)_t(\d+)_z(\d+)", name).groups())


def load(f):
    g = cv2.imread(f, cv2.IMREAD_GRAYSCALE)
    g = cv2.resize(g, (g.shape[1] // 2, g.shape[0] // 2), interpolation=cv2.INTER_AREA)
    return cv2.GaussianBlur(g, (0, 0), 2)


axis, files = sys.argv[1], [f for f in sys.argv[2:] if f.endswith(".jpg")]
R = 70
samples = []
for fa, fb in itertools.combinations(files, 2):
    pa, pb = pos(fa), pos(fb)
    n = pb[0] - pa[0] if axis == "pan" else pb[1] - pa[1]
    if n == 0:
        continue
    A, B = load(fa), load(fb)
    h, w = A.shape
    for cy in range(R + 10, h - R - 10, 60):
        for cx in range(R + 10, w - R - 10, 60):
            patch = A[cy - R:cy + R, cx - R:cx + R]
            if patch.std() < 12:
                continue
            res = cv2.matchTemplate(B, patch, cv2.TM_CCOEFF_NORMED)
            _, s, _, loc = cv2.minMaxLoc(res)
            if s < 0.93:
                continue
            # second-best check: reject repetitive texture
            res2 = res.copy()
            x0, y0 = loc
            res2[max(0, y0 - 20):y0 + 20, max(0, x0 - 20):x0 + 20] = -1
            if res2.max() > s - 0.03:
                continue
            dx, dy = loc[0] + R - cx, loc[1] + R - cy
            if axis == "pan":
                d, off, other = dx, dy, cy
                x = (cx + dx / 2 - w / 2) / w
            else:
                d, off = -dy, dx
                x = -(cy + dy / 2 - h / 2) / w
            if abs(d) < 8 or abs(off) > 0.15 * abs(d) + 6:
                continue
            samples.append((x, -n / (d / w)))
S = np.array(samples)
if len(S) < 5:
    print(f"{axis}: too few samples ({len(S)})")
    sys.exit(1)
# robust fit: iterate dropping >2.5 sigma
keep = np.ones(len(S), bool)
for _ in range(4):
    M = np.stack([np.ones(keep.sum()), 3 * S[keep, 0] ** 2], 1)
    coef, *_ = np.linalg.lstsq(M, S[keep, 1], rcond=None)
    r = np.stack([np.ones(len(S)), 3 * S[:, 0] ** 2], 1) @ coef - S[:, 1]
    sd = np.std(r[keep])
    keep = np.abs(r) < 2.5 * sd + 1e-9
for lo in np.arange(-0.5, 0.5, 0.1):
    sel = keep & (S[:, 0] >= lo) & (S[:, 0] < lo + 0.1)
    if sel.sum():
        print(f"  x {lo:+.1f}: n={sel.sum():4d} slope median {np.median(S[sel,1]):7.1f}", file=sys.stderr)
print(f"{axis} a={coef[0]:.1f} b={coef[1]:.1f} n={keep.sum()}/{len(S)} sd={sd:.1f} "
      f"edge(0.5)={coef[0]*0.5+coef[1]*0.125:.1f}")
