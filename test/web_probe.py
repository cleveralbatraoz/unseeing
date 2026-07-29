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
    """Minimal PNG reader: count pixels with any channel > 24."""
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    pos, w, h, idat = 8, 0, 0, b""
    while pos < len(data):
        ln = struct.unpack("!I", data[pos:pos + 4])[0]
        typ = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + ln]
        if typ == b"IHDR":
            w, h = struct.unpack("!II", body[:8])
        elif typ == b"IDAT":
            idat += body
        pos += 12 + ln
    raw = zlib.decompress(idat)
    stride = w * 4 + 1
    lit = 0
    for row in range(h):
        line = raw[row * stride + 1: (row + 1) * stride]
        for x in range(0, len(line), 4):
            if line[x] > 24 or line[x + 1] > 24 or line[x + 2] > 24:
                lit += 1
    return lit, w * h


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
