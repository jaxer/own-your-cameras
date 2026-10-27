"""Acceptance for cam::position3d on the camera: tap real features and measure where they land.

usage: validate.py PAN TILT ZOOM X Y [X Y ...]
  X, Y are pixels of a 1920x1440 frame at that pose. Without --mqtt the script computes the target
  itself with the same table as Optics::E1_ZOOM (table.py) and moves with absolute pan/tilt; with
  --mqtt it publishes the card's position3d payload and lets the daemon compute it.
For each point: grab a frame, move, grab again, template-match a patch around the point and report its
offset from the centre. Needs numpy and opencv-python-headless; shoot.sh/cam.sh environment.
"""
import json
import os
import subprocess
import sys

import cv2

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from table import TILT_RATIO, steps  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.environ.get("OUT", "/tmp/position3d-validate")


def run(*args):
    subprocess.run(["bash", *args], check=True, stdout=subprocess.DEVNULL)


def shoot(p, t, z, out):
    run(f"{HERE}/shoot.sh", str(p), str(t), str(z), out)


def state():
    out = subprocess.run(["bash", f"{HERE}/cam.sh", "state"], check=True, capture_output=True, text=True)
    words = out.stdout.split()  # "camera/<cam>/pan/state 5005 camera/<cam>/tilt/state 488 ..."
    return {topic.split("/")[-2]: int(v) for topic, v in zip(words[::2], words[1::2])}


args = [a for a in sys.argv[1:] if a != "--mqtt"]
mqtt = "--mqtt" in sys.argv
p0, t0, z = (int(v) for v in args[:3])
pts = [float(v) for v in args[3:]]
os.makedirs(OUT, exist_ok=True)
base = f"{OUT}/base.jpg"
shoot(p0, t0, z, base)
img = cv2.imread(base, cv2.IMREAD_GRAYSCALE)
H, W = img.shape
worst = 0.0
for i in range(0, len(pts), 2):
    px, py = pts[i], pts[i + 1]
    out = f"{OUT}/point{i // 2}.jpg"
    if mqtt:
        hx, hy = round(px / W * 255), round((1 - py / H) * 255)
        payload = {"start_x": hx, "start_y": hy, "end_x": hx, "end_y": hy, "operation": "zoomto"}
        run(f"{HERE}/cam.sh", "pub", "position3d", json.dumps(payload))
        subprocess.run(["sleep", "5"], check=True)
        run(f"{HERE}/cam.sh", "frame", out)
        s = state()
        p, t = s.get("pan"), s.get("tilt")
    else:
        x, y = (px - W / 2) / W, -(py - H / 2) / W
        p, t = round(p0 + steps(x, z)), round(t0 + TILT_RATIO * steps(y, z))
        shoot(p, t, z, out)
    got = cv2.imread(out, cv2.IMREAD_GRAYSCALE)
    r = 60
    patch = img[int(py) - r:int(py) + r, int(px) - r:int(px) + r]
    res = cv2.matchTemplate(got, patch, cv2.TM_CCOEFF_NORMED)
    _, score, _, loc = cv2.minMaxLoc(res)
    cx, cy = loc[0] + r, loc[1] + r
    ox, oy = (cx - W / 2) / W * 100, (cy - H / 2) / H * 100
    if score >= 0.85:
        worst = max(worst, abs(ox), abs(oy))
    print(f"point ({px:.0f},{py:.0f}) -> pan {p} tilt {t}: lands at ({cx},{cy}), "
          f"off centre ({ox:+.1f}%, {oy:+.1f}%) match {score:.2f}{'' if score >= 0.85 else ' (unreliable)'}")
    shoot(p0, t0, z, base)
print(f"worst reliable offset {worst:.1f}% of the frame")
