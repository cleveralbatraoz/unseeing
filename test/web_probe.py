#!/usr/bin/env python3
"""Probe a running headless Chrome over the DevTools protocol (stdlib only):
wait real seconds for the wasm engine to boot, then assert the Godot loader
overlay is gone and the canvas shows lit pixels (the ?demo tap makes waves).
Usage: web_probe.py <devtools_port> <wait_seconds>
Exit codes: 0 pass, 1 fail."""
import base64
import json
import os
import socket
import struct
import sys
import time
import urllib.request
import zlib
from urllib.parse import urlsplit, urlunsplit

port, wait_s = int(sys.argv[1]), float(sys.argv[2])

target = None
for _ in range(30):
    try:
        pages = json.load(urllib.request.urlopen(
            f"http://127.0.0.1:{port}/json/list", timeout=2))
        target = next((p for p in pages if p.get("type") == "page"), None)
        if target:
            break
    except Exception:
        pass
    time.sleep(1)
if not target:
    print("smoke: FAIL — no page target")
    sys.exit(1)
path = target["webSocketDebuggerUrl"].split(f":{port}")[1]

sock = socket.create_connection(("127.0.0.1", port), timeout=120)
key = base64.b64encode(os.urandom(16)).decode()
sock.sendall((
    f"GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n"
    f"Upgrade: websocket\r\nConnection: Upgrade\r\n"
    f"Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
).encode())
resp = b""
while b"\r\n\r\n" not in resp:
    resp += sock.recv(4096)
assert b"101" in resp.split(b"\r\n")[0], resp[:200]


def send(obj):
    data = json.dumps(obj).encode()
    mask = os.urandom(4)
    masked = bytes(b ^ mask[i % 4] for i, b in enumerate(data))
    ln = len(data)
    if ln < 126:
        hdr = struct.pack("!BB", 0x81, 0x80 | ln)
    elif ln < 65536:
        hdr = struct.pack("!BBH", 0x81, 0x80 | 126, ln)
    else:
        hdr = struct.pack("!BBQ", 0x81, 0x80 | 127, ln)
    sock.sendall(hdr + mask + masked)


def recv_exact(n):
    buf = b""
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        assert chunk, "socket closed"
        buf += chunk
    return buf


def recv_msg():
    payload = b""
    while True:
        b1, b2 = struct.unpack("!BB", recv_exact(2))
        ln = b2 & 0x7F
        if ln == 126:
            ln = struct.unpack("!H", recv_exact(2))[0]
        elif ln == 127:
            ln = struct.unpack("!Q", recv_exact(8))[0]
        if b2 & 0x80:
            mask = recv_exact(4)
            data = bytes(x ^ mask[i % 4] for i, x in enumerate(recv_exact(ln)))
        else:
            data = recv_exact(ln)
        if (b1 & 0x0F) == 0x9:  # ping
            continue
        payload += data
        if b1 & 0x80:
            return payload


time.sleep(wait_s)
send({"id": 1, "method": "Runtime.evaluate", "params": {
    "expression": "(function(){var s=document.getElementById('status');"
                  "return s?getComputedStyle(s).visibility:'gone'})()",
    "returnByValue": True}})
send({"id": 2, "method": "Page.captureScreenshot", "params": {"format": "png"}})
status = shot = None
deadline = time.time() + 60
while (status is None or shot is None) and time.time() < deadline:
    msg = json.loads(recv_msg())
    if msg.get("id") == 1:
        status = msg.get("result", {}).get("result", {}).get("value")
    if msg.get("id") == 2:
        shot = msg["result"]["data"]

png = base64.b64decode(shot or "")


## Bytes per pixel for each PNG colour type this reader accepts (0 = grey,
## 2 = truecolor RGB, 6 = truecolor+alpha RGBA) — indexed by the IHDR byte.
_PNG_CHANNELS = {0: 1, 2: 3, 6: 4}


def _paeth(a, b, c):
    """The PNG Paeth predictor (spec §9.4): pick whichever of the left,
    above, or above-left reconstructed byte is numerically closest to
    a + b - c."""
    p = a + b - c
    pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c


def decode_png(data):
    """Minimal PNG reader: returns (width, height, DEFILTERED scanline
    bytes — one leading zero byte per row, kept only so downstream slicing
    stays `row * stride + 1 : (row + 1) * stride` — channels, stride).

    Page.captureScreenshot does NOT always hand back RGBA — Chrome drops
    the alpha channel (colour type 2, 3 bytes/pixel) whenever the captured
    surface is fully opaque, which this game's canvas always is (CSS pins
    its background to opaque black), so an assumed 4-bytes/pixel stride
    silently misreads every row. Read the real type instead of assuming
    one.

    A PNG scanline also picks its OWN filter (spec §9, one of 5 types,
    chosen per row to help compression) and is meaningless until that
    filter is reversed. An earlier version of this reader skipped only the
    leading filter-type byte and treated the rest as raw pixels — correct
    only for filter 0 (None). Measured against real captures from this
    exact gate: Chrome's encoder does NOT always choose 0 — a genuine
    capture was seen using filter 3 (Average) — so every standard filter
    (0-4) is reconstructed for real below, per the PNG spec's own
    recurrence (each row references the row above it, both BEFORE this
    row's own filter is applied). A filter byte outside 0-4 is invalid PNG
    and fails loudly, naming the row and the byte found, rather than
    guessing."""
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    pos, w, h, colortype, idat = 8, 0, 0, None, b""
    while pos < len(data):
        ln = struct.unpack("!I", data[pos:pos + 4])[0]
        typ = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + ln]
        if typ == b"IHDR":
            w, h = struct.unpack("!II", body[:8])
            colortype = body[9]
        elif typ == b"IDAT":
            idat += body
        pos += 12 + ln
    assert colortype in _PNG_CHANNELS, f"unsupported PNG colour type {colortype}"
    channels = _PNG_CHANNELS[colortype]
    filtered = zlib.decompress(idat)
    row_bytes = w * channels
    stride = row_bytes + 1
    prev = bytearray(row_bytes)  # the "row above row 0" is defined as all zero
    out = bytearray(stride * h)
    for row in range(h):
        base = row * stride
        filt = filtered[base]
        cur = bytearray(filtered[base + 1: base + stride])
        if filt == 0:  # None
            pass
        elif filt == 1:  # Sub
            for x in range(channels, row_bytes):
                cur[x] = (cur[x] + cur[x - channels]) & 0xFF
        elif filt == 2:  # Up
            for x in range(row_bytes):
                cur[x] = (cur[x] + prev[x]) & 0xFF
        elif filt == 3:  # Average
            for x in range(row_bytes):
                a = cur[x - channels] if x >= channels else 0
                cur[x] = (cur[x] + (a + prev[x]) // 2) & 0xFF
        elif filt == 4:  # Paeth
            for x in range(row_bytes):
                a = cur[x - channels] if x >= channels else 0
                c = prev[x - channels] if x >= channels else 0
                cur[x] = (cur[x] + _paeth(a, prev[x], c)) & 0xFF
        else:
            raise AssertionError(
                f"PNG row {row} carries filter byte {filt}, which is not a "
                "valid PNG filter type (0-4) — corrupt or truncated data"
            )
        out[base + 1: base + stride] = cur
        prev = cur
    return w, h, bytes(out), channels, stride


def count_lit(data):
    """Count pixels with any channel > 24, out of the total pixel count."""
    w, h, raw, channels, stride = decode_png(data)
    lit = 0
    for row in range(h):
        line = raw[row * stride + 1: (row + 1) * stride]
        for x in range(0, len(line), channels):
            if any(c > 24 for c in line[x:x + min(channels, 3)]):
                lit += 1
    return lit, w * h


def g_channel_levels(data, bucket=8, min_pixels=6):
    """Distinct quantized G-channel byte values, each covering at least
    min_pixels pixels (a handful of anti-aliased seam pixels at one
    real boundary must not count as a second "level"), ignoring G == 0
    (background the camera never touched). Bucketed by floor division so
    1-LSB rounding noise inside one flat face cannot inflate the count."""
    w, h, raw, channels, stride = decode_png(data)
    assert channels >= 2, "no G channel in a greyscale screenshot"
    counts = {}
    for row in range(h):
        line = raw[row * stride + 1: (row + 1) * stride]
        for x in range(0, len(line), channels):
            g = line[x + 1]
            if g > 0:
                key = g // bucket
                counts[key] = counts.get(key, 0) + 1
    return {k for k, v in counts.items() if v >= min_pixels}


lit, total = count_lit(png)
overlay_gone = status in ("hidden", "gone", None)
print(f"smoke: loader_overlay={status} lit_pixels={lit}/{total}")
if not overlay_gone:
    print("smoke: FAIL — engine never finished booting (loader still visible)")
    sys.exit(1)
if lit < 40:
    print("smoke: FAIL — canvas is effectively black; the demo tap revealed nothing")
    sys.exit(1)
print("smoke: PASS")

# --- G-channel readback: does CUSTOM0 actually reach G on the one platform
# a browser can inspect from the outside? ------------------------------
#
# The label the data pass paints into G never reaches the FINAL composited
# image on its own: hearing_post.gdshader broadcasts one scalar into R, G
# and B alike (`vec3 col = vec3(edge * reveal)`, hearing_post.gdshader:93),
# so a screenshot of the normal game shows nothing G-specific — R, G and B
# are always equal there, healthy or dead alike. The only way to see the
# raw per-vertex label from OUTSIDE the engine is to look BEHIND the
# hearing pass, exactly as tests/probe/occlusion_probe.gd's `_hide_quad`
# already does natively for the windowed GPU probe: reload with `&gprobe`
# appended, which tells main.gd (see its `_post_quad` field) to hide its
# own post-processing quad so the data pass's own unshaded ALBEDO —
# (reveal, label, distance) packed by data_core.gdshaderinc's pack_data —
# reaches the screen directly, camera and all. A healthy label channel
# shows several distinct values wherever the camera sees more than one
# labelled surface (walls, floor, ceiling never share a label); a dead
# CUSTOM0 binding shows exactly one value everywhere, no matter the
# geometry in view.
parts = urlsplit(target["url"])
query = (parts.query + "&gprobe") if parts.query else "gprobe"
gprobe_url = urlunsplit((parts.scheme, parts.netloc, parts.path, query, parts.fragment))

_msg_id = [2]


def ws_call(method, params=None, timeout=30):
    """Send one CDP request with a fresh id and block for its matched
    reply, discarding any other traffic that arrives first (events,
    stale ids) the same way the two hand-matched calls above already do."""
    _msg_id[0] += 1
    mid = _msg_id[0]
    send({"id": mid, "method": method, "params": params or {}})
    deadline = time.time() + timeout
    while time.time() < deadline:
        msg = json.loads(recv_msg())
        if msg.get("id") == mid:
            return msg.get("result", {})
    raise TimeoutError(f"no reply to {method} (id {mid}) within {timeout}s")


ws_call("Page.navigate", {"url": gprobe_url})

# Condition-based, not a guessed sleep: poll #status until it is truly GONE
# (index.html's own loader calls statusOverlay.remove() — see
# game/build/web/index.html's setStatusMode — so "gone" means
# getElementById returns null). "hidden" is NOT an acceptable stand-in
# here: #status's CSS default, before the loader has drawn a single frame,
# IS visibility:hidden, so accepting it races a fresh reload — the poll can
# read that pre-load default and return before the reload even started.
# window.location.search is checked too, so a stale read from the PAGE
# BEFORE this navigation cannot be mistaken for readiness either.
CHECK_EXPR = (
    "(function(){var s=document.getElementById('status');"
    "var vis=s?getComputedStyle(s).visibility:'gone';"
    "return JSON.stringify({vis:vis, loc:window.location.search});})()"
)
g_overlay = None
for _ in range(30):
    val = ws_call(
        "Runtime.evaluate", {"expression": CHECK_EXPR, "returnByValue": True}
    ).get("result", {}).get("value")
    parsed = json.loads(val) if val else {}
    g_overlay = parsed.get("vis")
    if "gprobe" in (parsed.get("loc") or "") and g_overlay == "gone":
        break
    time.sleep(1)
else:
    print("smoke: FAIL — the ?gprobe reload never finished booting")
    sys.exit(1)

# Still condition-based: a screenshot taken the instant "gone" is observed
# can catch the browser's own compositor mid-catch-up after a full-page
# navigation (measured — the first post-navigation frame or two can still
# show the PREVIOUS page's content). Poll screenshots until two consecutive
# reads agree on the G-channel levels they carry, rather than guessing how
# long that catch-up takes.
prev_levels = None
levels = None
for _ in range(10):
    shot2 = ws_call("Page.captureScreenshot", {"format": "png"}).get("data")
    png2 = base64.b64decode(shot2 or "")
    levels = g_channel_levels(png2)
    if levels == prev_levels:
        break
    prev_levels = levels
    time.sleep(1)

print(f"smoke: gprobe_overlay={g_overlay} g_levels={sorted(levels)}")
if len(levels) <= 1:
    print(
        "smoke: FAIL — the G channel reads back a single value; "
        "CUSTOM0 is not binding to G on web"
    )
    sys.exit(1)
print("smoke: PASS (G channel)")
