import sys
import time
import math
import json

delay = 0.025

def sin_range(lower, upper):
    range = upper - lower
    def sin_range_impl(x):
        return range * (0.5 * math.sin(x) + 0.5)

    return sin_range_impl

def identity(x): return x

def offset(offset): return lambda x: x - offset

channels = {
    "RPM": sin_range(0, 6500),
    "ECT": sin_range(0, 230),
    "OIL_PRESSURE": sin_range(0, 60),
    "SESSION_TIME": offset(time.time())
}


while True:
    t = time.time()
    sys.stdout.write(json.dumps({k: channels[k](t) for k in sorted(channels)}))
    sys.stdout.write('\n')
    sys.stdout.flush()
    time.sleep(delay)
