import sys
import time
import math
import json

delay = 0.1

channels = {
    "RPM": (0, 6500),
    "ECT": (0, 300),
    "OIL_PRESSURE": (0, 60),
    "SESSION_TIME": (0, 60)
}

def value(k, t):
    l, u = channels[k]
    r = u - l
    return r * math.sin(time.time()) + l

while True:
    t = time.time()
    sys.stdout.write(json.dumps({k: channels[k](t) for k in sorted(channels)}))
    sys.stdout.write('\n')
    sys.stdout.flush()
    time.sleep(delay)
