import os, sys, time
total, pace_ms = int(sys.argv[1]), int(sys.argv[2])
sequence = 0
remaining = total
while remaining:
    rows = []
    for _ in range(256):
        prefix = (f"RESOURCE row {sequence:08d} alpha beta 東京 ").encode()
        rows.append(prefix + b"." * (127 - len(prefix)) + b"\n")
        sequence += 1
    block = b"".join(rows)
    chunk = memoryview(block)[:min(len(block), remaining)]
    while chunk:
        written = os.write(1, chunk)
        chunk = chunk[written:]
        remaining -= written
    if pace_ms:
        time.sleep(pace_ms / 1000)
os.write(1, b"RESOURCE_DONE\n")
