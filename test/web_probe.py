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


# --- decode_png/_paeth self-test -----------------------------------------
#
# decode_png reconstructs the PNG filters that back every pixel assertion
# this suite makes (count_lit, g_channel_levels), and nothing else in the
# repo touches it: cargo and gdUnit never run Python, so a regression here
# has no other net under it. That is exactly the gap the strict-TDD rule
# for gate code exists to close.
#
# Every expected byte list below was HAND-DERIVED from the PNG spec's own
# recurrence — Recon(x) = Filt(x) + f(a, b, c), where f depends on the
# filter type and a/b/c (left, above, above-left) are 0 at an edge — and
# cross-checked against an independent, differently-shaped reimplementation
# before being hardcoded here, never computed by calling decode_png itself
# (a mirror assertion passes no matter what the code under test does).

def _png_chunk(typ, body):
    """One PNG chunk: length, type, body, and a zeroed CRC. decode_png does
    not validate the CRC (nor does extracting pixel data require it), so a
    real CRC32 implementation is not a dependency this self-test needs."""
    return struct.pack("!I", len(body)) + typ + body + b"\x00\x00\x00\x00"


def _build_png(colortype, width, rows):
    """Hand-encode a minimal, valid PNG from (filter_byte, [filtered pixel
    bytes]) rows — the exact wire format decode_png parses."""
    height = len(rows)
    ihdr = struct.pack("!IIBBBBB", width, height, 8, colortype, 0, 0, 0)
    raw = b"".join(bytes([filt]) + bytes(pixels) for filt, pixels in rows)
    idat = zlib.compress(raw)
    return (
        b"\x89PNG\r\n\x1a\n"
        + _png_chunk(b"IHDR", ihdr)
        + _png_chunk(b"IDAT", idat)
        + _png_chunk(b"IEND", b"")
    )


def _rows_from_decoded(w, h, raw, channels, stride):
    """Peel decode_png's own leading-byte-per-row convention back into a
    plain list of pixel-byte lists, one per row."""
    return [list(raw[r * stride + 1:(r + 1) * stride]) for r in range(h)]


## (name, colour type, width, filtered rows, expected reconstructed rows).
_SELFTEST_FIXTURES = [
    # Sub (filter 1): left-neighbour recurrence, including the column-0
    # edge (no left neighbour, so a = 0).
    ("filter_sub_1ch", 0, 3,
     [(1, [5, 3, 2])],
     [[5, 8, 10]]),
    # Up (filter 2) on ROW 0 ITSELF: the "row above row 0" is defined as
    # all-zero, so this exercises that edge directly, then a second row
    # against a real previous row.
    ("filter_up_1ch_row0_edge", 0, 2,
     [(2, [7, 9]), (2, [3, 4])],
     [[7, 9], [10, 13]]),
    # Average (filter 3) on row 0: both edges at once (a = 0 at column 0,
    # b = 0 for the whole row since there is no row above).
    ("filter_average_1ch_row0_and_col0_edges", 0, 3,
     [(3, [10, 10, 10])],
     [[10, 15, 17]]),
    # Paeth (filter 4): one fixture per predictor branch, so a mutated
    # comparison operator is caught by SOME case even though the branches
    # a mutation could shift between are otherwise adjacent.
    ("paeth_a_wins", 0, 2,
     [(0, [90, 100]), (4, [176, 45])],
     [[90, 100], [10, 55]]),
    ("paeth_b_wins", 0, 2,
     [(0, [90, 10]), (4, [10, 67])],
     [[90, 10], [100, 77]]),
    ("paeth_c_wins", 0, 2,
     [(0, [83, 85]), (4, [253, 7])],
     [[83, 85], [80, 90]]),
    # The canonical Paeth tie-break case (a == b == c exactly, at x=1):
    # every implementation of the predictor must resolve this without
    # crashing or picking an out-of-range byte, per the spec's own
    # tie-break order (a, then b, then c).
    ("paeth_tie_a_eq_b_eq_c", 0, 3,
     [(0, [50, 50, 50]), (4, [0, 7, 3])],
     [[50, 50, 50], [50, 57, 60]]),
    # Multi-channel: Sub across a 3-channel (RGB, colour type 2) row —
    # the left neighbour is bpp=3 bytes back, not 1, so this is where a
    # per-channel stride bug in the SUB branch shows up. Sub is one of
    # three branches that read a strided neighbour; the other two are
    # covered further down.
    ("rgb_3channel_sub", 2, 2,
     [(1, [10, 20, 30, 5, 6, 7])],
     [[10, 20, 30, 15, 26, 37]]),
    # Multi-channel: Up across a 4-channel (RGBA, colour type 6) row.
    ("rgba_4channel_up", 6, 2,
     [(0, [1, 2, 3, 4, 5, 6, 7, 8]), (2, [10, 10, 10, 10, 1, 1, 1, 1])],
     [[1, 2, 3, 4, 5, 6, 7, 8], [11, 12, 13, 14, 6, 7, 8, 9]]),
    # Multi-channel AVERAGE and PAETH — the combination real captures
    # actually take, and the one the fixtures above miss entirely. Both of
    # those branches read a neighbour `channels` bytes back, and every
    # other Average/Paeth fixture here is colour type 0, where `x -
    # channels` and `x - 1` are the same expression. Chrome hands this
    # gate colour type 2 (see decode_png's own docstring) and picks a
    # filter per row, so a stride bug in exactly these two branches would
    # misdecode production screenshots while the self-test stayed green.
    #
    # Average, RGB: row 0 is unfiltered, so row 1 exercises a real `b`
    # (above) alongside the strided `a` (left). At x = 3 the true left
    # neighbour is byte 0 (6), not byte 2 (18) — 4 + (6+40)//2 = 27, where
    # the 1-byte stride would give 4 + (18+40)//2 = 33.
    ("rgb_3channel_average", 2, 2,
     [(0, [10, 20, 30, 40, 50, 60]), (3, [1, 2, 3, 4, 5, 6])],
     [[10, 20, 30, 40, 50, 60], [6, 12, 18, 27, 36, 45]]),
    # Paeth reads TWO strided neighbours — `a` from this row and `c` from
    # the row above — so it takes two fixtures to pin both.
    #
    # `a`: a uniform row above makes b == c, which drives the predictor
    # straight to `a` (p = a + b - c = a, so pa = 0 wins outright). At
    # x = 3 that is byte 0 (101), giving 4 + 101 = 105; a 1-byte stride
    # would read byte 2 (103) and give 107.
    ("rgb_3channel_paeth_left_stride", 2, 2,
     [(0, [100, 100, 100, 100, 100, 100]), (4, [1, 2, 3, 4, 5, 6])],
     [[100, 100, 100, 100, 100, 100], [101, 102, 103, 105, 107, 109]]),
    # `c`: here the left neighbour is 10 under EITHER stride, so only the
    # above-left byte can move the answer — and it moves it across a
    # branch, not by a little. True c = prev[0] = 200 gives
    # paeth(10, 200, 200) = a = 10; a 1-byte stride reads prev[2] = 10 and
    # gives paeth(10, 200, 10) = b = 200.
    ("rgb_3channel_paeth_upleft_stride", 2, 2,
     [(0, [200, 5, 10, 200, 60, 70]), (4, [66, 0, 0, 0, 0, 0])],
     [[200, 5, 10, 200, 60, 70], [10, 5, 10, 10, 60, 70]]),
    # All five filter types, mixed across consecutive rows of ONE image —
    # the shape a real screenshot actually takes (Chrome picks a filter
    # per row independently), not five isolated single-row PNGs.
    ("mixed_all_five_filters_per_row", 0, 3,
     [(0, [100, 110, 120]), (1, [5, 3, 2]), (2, [1, 1, 1]),
      (3, [2, 2, 2]), (4, [3, 3, 3])],
     [[100, 110, 120], [5, 8, 10], [6, 9, 11], [5, 9, 12], [8, 12, 15]]),
]


def _self_test_decode_png():
    """Run every fixture above plus the invalid-filter-byte raise path.
    Prints one ok/not ok line per case (TAP-ish, matching this repo's
    native probes) and returns True only if every case passed."""
    passed = failed = 0
    for name, colortype, width, filt_rows, expected in _SELFTEST_FIXTURES:
        fixture_png = _build_png(colortype, width, filt_rows)
        try:
            w, h, raw, channels, stride = decode_png(fixture_png)
            got = _rows_from_decoded(w, h, raw, channels, stride)
        except Exception as exc:
            print(f"not ok - {name}: decode_png raised unexpectedly: {exc}")
            failed += 1
            continue
        if got == expected:
            print(f"ok - {name}")
            passed += 1
        else:
            print(f"not ok - {name}: expected {expected}, got {got}")
            failed += 1

    # A filter byte outside 0-4 is invalid PNG and MUST raise loudly,
    # naming the row and the byte found — never silently misdecode.
    bad_png = _build_png(0, 2, [(5, [1, 2])])
    try:
        decode_png(bad_png)
        print("not ok - invalid_filter_byte_raises: decode_png did not raise")
        failed += 1
    except AssertionError as exc:
        if "row 0" in str(exc) and "filter byte 5" in str(exc):
            print("ok - invalid_filter_byte_raises")
            passed += 1
        else:
            print(f"not ok - invalid_filter_byte_raises: wrong message: {exc}")
            failed += 1
    except Exception as exc:
        print(f"not ok - invalid_filter_byte_raises: wrong exception type: {exc!r}")
        failed += 1

    print(f"self-test: {passed} ok, {failed} failed")
    return failed == 0


if len(sys.argv) > 1 and sys.argv[1] == "--selftest":
    sys.exit(0 if _self_test_decode_png() else 1)

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


def data_pass_split(data, lit_floor=8, split=8):
    """(pixels whose G differs from their R, non-black pixels) — the
    witness that a screenshot really is the DATA pass and not the
    composited game.

    The composite cannot produce a single such pixel. hearing_post.gdshader
    builds its whole output from `vec3(scalar)` terms — `vec3(edge *
    reveal)`, the per-pulse `col += vec3(...)`, the `vec3(0.006)` void, a
    scalar vignette multiply and a scalar grain add — so R, G and B are
    EQUAL in every composited pixel, healthy or dead. The data pass packs
    three different quantities instead (`data_core.gdshaderinc::pack_data`:
    R reveal, G label, B distance), so its pixels disagree.

    `split` and `lit_floor` are tolerances, not thresholds: the composite's
    channels are equal exactly, so any positive `split` separates the two
    images, and `lit_floor` only drops the untouched background the camera
    never wrote to."""
    w, h, raw, channels, stride = decode_png(data)
    assert channels >= 3, "no R/G split to measure in a greyscale screenshot"
    apart = seen = 0
    for row in range(h):
        line = raw[row * stride + 1: (row + 1) * stride]
        for x in range(0, len(line), channels):
            r, g, b = line[x], line[x + 1], line[x + 2]
            if max(r, g, b) <= lit_floor:
                continue
            seen += 1
            if abs(g - r) > split:
                apart += 1
    return apart, seen


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
# appended, which tells UnseeingGame (see `setup_post_quad` in
# rust/src/nodes/game.rs) to hide its own post-processing quad so the data
# pass's own unshaded ALBEDO —
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

# WHICH IMAGE IS THIS? The level count below is only meaningful on the data
# pass. The composite is guaranteed to carry many G levels — its void term
# lights every pixel and the vignette and grain spread that across dozens
# of buckets — so if the ?gprobe hide (`UnseeingGame::setup_post_quad`) stops
# firing, the probe would screenshot the COMPOSITE and report PASS for a
# gate that had stopped looking at CUSTOM0 at all. Two conditions cannot be
# allowed to collapse into one: this repo has paid for a silently green
# gate twice already (deploy.sh reading UNSEEING_BUILD back off the live
# page; gdUnit4's empty run).
#
# One in eight is the floor, and it is deliberately far below what the data
# pass produces rather than tight against it. Measured on this gate, same
# build, two runs: the data pass split 87056/184320 (47%) and
# 184097/184320 (99.9%) — it swings with how much of the room the demo tap
# has revealed by capture time — while the COMPOSITE captured in those same
# two runs split 0/21365 and 0/21332. Exactly zero, both times, as the
# shader says it must. There is no threshold to tune between those two
# populations; the only real risk is a floor high enough to redden a
# healthy run, so it sits well under the lower observation.
apart, seen = data_pass_split(png2)
print(f"smoke: data_pass_split={apart}/{seen}")
if seen == 0 or apart * 8 < seen:
    print(
        "smoke: FAIL — this screenshot is the composited game, not the data "
        "pass: R and G agree almost everywhere, which only hearing_post can "
        "produce. The ?gprobe post-quad hide is not firing, so the G reading "
        "below would prove nothing about CUSTOM0."
    )
    sys.exit(1)

print(f"smoke: gprobe_overlay={g_overlay} g_levels={sorted(levels)}")
if len(levels) <= 1:
    print(
        "smoke: FAIL — the G channel reads back a single value; "
        "CUSTOM0 is not binding to G on web"
    )
    sys.exit(1)
print("smoke: PASS (G channel)")
