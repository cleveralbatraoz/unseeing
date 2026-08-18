"""Read the web platform probe's verdicts out of a running headless browser.

The probe scene (game/tests/probe/platform_probe.gd) reports the same
answers twice, on purpose:

  - it PRINTS them, which Godot's web build forwards to the JS console; and
  - it RENDERS them, as three regions of the frame.

This reads the console. That is the cheaper of the two and it is sufficient
*if* the printed numbers are trustworthy — which they are only when Godot's
in-game framebuffer readback works on this platform, since that is how the
scene reaches its own pixels. So the control is what decides whether to
believe the run at all: the probe prints a control value the write pass put
at a known place, and if that does not come back as 0.5 the readback is not
working and every other number in the run is void rather than merely wrong.

Usage: platform_probe_web.py <devtools-port> [seconds]
"""

import json
import socket
import sys
import time

port = int(sys.argv[1])
budget = float(sys.argv[2]) if len(sys.argv) > 2 else 90.0


def devtools_target(deadline):
    """The page's WebSocket URL, once the browser is answering."""
    import urllib.error
    import urllib.request

    while time.monotonic() < deadline:
        try:
            raw = urllib.request.urlopen(
                "http://127.0.0.1:%d/json/list" % port, timeout=2
            ).read()
            for entry in json.loads(raw):
                if entry.get("type") == "page" and entry.get("webSocketDebuggerUrl"):
                    return entry["webSocketDebuggerUrl"]
        except (urllib.error.URLError, OSError, ValueError):
            pass
        time.sleep(0.1)
    return None


deadline = time.monotonic() + budget
ws_url = devtools_target(deadline)
if not ws_url:
    print("platform-web: FAILED the browser's DevTools endpoint never answered")
    sys.exit(1)

host_port, path = ws_url.split("://", 1)[1].split("/", 1)
host, sock_port = host_port.split(":")
sock = socket.create_connection((host, int(sock_port)), timeout=10)
sock.sendall(
    (
        "GET /%s HTTP/1.1\r\nHost: %s\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
        "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
        % (path, host_port)
    ).encode()
)
# drain the handshake response
buf = b""
while b"\r\n\r\n" not in buf:
    chunk = sock.recv(4096)
    if not chunk:
        print("platform-web: FAILED the browser closed the DevTools socket")
        sys.exit(1)
    buf += chunk


def send(obj):
    body = json.dumps(obj).encode()
    header = bytearray([0x81])
    n = len(body)
    if n < 126:
        header.append(0x80 | n)
    elif n < 65536:
        header.append(0x80 | 126)
        header += n.to_bytes(2, "big")
    else:
        header.append(0x80 | 127)
        header += n.to_bytes(8, "big")
    header += b"\x00\x00\x00\x00"  # mask key of zero: the payload is its own mask
    sock.sendall(bytes(header) + body)


def recv_exact(n):
    out = b""
    while len(out) < n:
        chunk = sock.recv(n - len(out))
        if not chunk:
            raise OSError("socket closed")
        out += chunk
    return out


def recv_msg():
    payload = b""
    while True:
        b0, b1 = recv_exact(2)
        length = b1 & 0x7F
        if length == 126:
            length = int.from_bytes(recv_exact(2), "big")
        elif length == 127:
            length = int.from_bytes(recv_exact(8), "big")
        data = recv_exact(length) if length else b""
        if (b1 & 0x0F) == 0x9:  # ping
            continue
        payload += data
        if b0 & 0x80:
            return payload


send({"id": 1, "method": "Runtime.enable"})
send({"id": 2, "method": "Log.enable"})

lines = []
sock.settimeout(2.0)
while time.monotonic() < deadline:
    try:
        msg = json.loads(recv_msg())
    except (OSError, ValueError):
        continue
    method = msg.get("method")
    text = None
    if method == "Runtime.consoleAPICalled":
        parts = msg["params"].get("args", [])
        text = " ".join(str(a.get("value", "")) for a in parts)
    elif method == "Log.entryAdded":
        text = msg["params"].get("entry", {}).get("text", "")
    if text and text.strip().startswith("#"):
        line = text.strip()
        if line not in lines:
            lines.append(line)
            print(line)
        if line.startswith("# CONTROL"):
            break

if not lines:
    print("platform-web: FAILED the page produced no probe output at all")
    sys.exit(1)

verdict = next((line for line in lines if line.startswith("# platform:")), None)
if verdict is None:
    print("platform-web: FAILED the probe never reported a verdict line")
    sys.exit(1)

# "# platform: levels 1024 ; depth 0.9490 (...) ; control 0.5020"
fields = verdict.replace(";", " ").split()
levels = int(fields[fields.index("levels") + 1])
depth = float(fields[fields.index("depth") + 1])
control = float(fields[-1])

ok = True
if abs(control - 0.5) > 0.02:
    print(
        "platform-web: FAILED the control read %.4f, not 0.5 — Godot's in-game "
        "readback is not working on this platform, so every number above is "
        "void rather than merely wrong" % control
    )
    ok = False
else:
    print("ok - the control reads %.4f, so the readback is trustworthy" % control)
    print("ok - the web screen texture holds %d levels per channel" % levels)
    print(
        "ok - the web depth texture reads %.4f (dead would be 0.0000)" % depth
        if depth > 0.01
        else "note - the web depth texture reads %.4f: DEAD here" % depth
    )

print(
    "# NOTE: measured under SwiftShader, the same software rasteriser "
    "test/web_smoke.sh uses. It executes the real GLSL but is not a real "
    "WebGL2 driver; treat these as the floor a browser guarantees, not as "
    "what a GPU-backed browser gives."
)
sys.exit(0 if ok else 1)
