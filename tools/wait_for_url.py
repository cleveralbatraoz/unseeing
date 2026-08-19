"""Block until a URL answers, or a deadline passes.

Polling in one process rather than a shell loop: a retry costs a socket
attempt instead of an interpreter start-up, and the bound is a real
deadline rather than an iteration count that means different wall-clock
time on every machine. test/web_smoke.sh inlines the same loop; this is
the copy the platform probe uses.
"""

import sys
import time
import urllib.error
import urllib.request

url, budget = sys.argv[1], float(sys.argv[2])
deadline = time.monotonic() + budget
while time.monotonic() < deadline:
    try:
        urllib.request.urlopen(url, timeout=1).read(1)
        sys.exit(0)
    except (urllib.error.URLError, OSError):
        time.sleep(0.05)
sys.exit(1)
