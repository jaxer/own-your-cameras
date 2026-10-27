"""The E1 Zoom map, mirrored from cam::position3d::Optics::E1_ZOOM (keep both in step)."""
import numpy as np

# (zoom, a, b): pan steps to a feature x frame widths off centre = a x + b x^3
TABLE = [(0, 1960.7, 553.4), (256, 1803.0, 385.2), (512, 1658.4, 217.7), (768, 1547.6, 127.3), (1024, 1462.6, 149.9)]
TILT_RATIO = 0.91


def steps(x, z):
    zs = [r[0] for r in TABLE]
    a = np.interp(z, zs, [r[1] for r in TABLE])
    b = np.interp(z, zs, [r[2] for r in TABLE])
    return a * x + b * x**3
