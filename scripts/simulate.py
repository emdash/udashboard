#! /usr/bin/python

"""
simulate.py

Usage: simulate.py [-d <delay>] CHANNEL_SPEC...

CHANNEL_SPEC:
  identity <name>
  sin      <name> <lower bound> <upper bound>
  const    <name> <value>
  random   <lower bound> <upper bound>

"""

import sys
import time
import math
import json
import random


def parse_identity(argv):
    return ((argv[0], lambda x: x), argv[1:])

def parse_sin(argv):
    name = argv[0]
    lower = float(argv[1])
    upper = float(argv[2])
    r = abs(upper - lower)
    return ((name, lambda x: r * (0.5 * math.sin(x) + 0.5)), argv[3:])

def parse_const(argv):
    name = argv[0]
    value = float(argv[1])
    return ((name, lambda x: value), argv[2:])

def parse_offset(argv):
    name = argv[1]
    value = float(argv[1])
    return ((name, lambda: x + offset), argv[2:])

def parse_rand(argv):
    name = argv[0]
    lower = float(argv[1])
    upper = float(argv[2])
    return ((name, lambda x: random.randrange(lower, upper)), argv[3:])

def parse_channels(argv):
    channels = {}
    while argv:
        token = argv[0]

        if token == "--identity":
            channel = parse_identity(argv[1:])
        elif token == "--sin":
            channel = parse_sin(argv[1:])
        elif token == "--const":
            channel = parse_const(argv[1:])
        elif token == "--rand":
            channel = parse_rand(argv[1:])
        else:
            raise SyntaxError(
                "Unexpected token " + repr(token) +
                "Expected one of --identity, --sin, --const, --rand"
            )

        ((name, func), argv) = channel
        channels[name] = func

    return channels

def parse_args(argv):
    if argv[0] == "-d":
        return (float(argv[1]), parse_channels(argv[2:]))
    else:
        return (0.025, parse_channels(argv))

try:
    delay, channels = parse_args(sys.argv[1:])
except BaseException, e:
    print __doc__
    exit(-1)


start = time.time()
while True:
    t = time.time() - start
    sys.stdout.write(json.dumps({k: channels[k](t) for k in sorted(channels)}))
    sys.stdout.write('\n')
    sys.stdout.flush()
    time.sleep(delay)
