#! /usr/bin/python

"""
replay.py

Usage: replay.py [options] <file>

-d <delay>, --delay=<delay>  Delay for <delay> seconds between samples.
-r, --repeat                 Start over again after EOF
"""

import sys
import time
import json
import itertools
from docopt import docopt


args = docopt(__doc__)

try:
    path = args['<file>']
except BaseException:
    print "No input file given."
    exit(-1)

try:
    data = open(path, "r")
except BaseException:
    print "Couldn't open %s!" % path
    exit(-1)

data_iter = data
try:
    if args['--repeat']:
        print >> sys.stderr, "Looping forever."
        data_iter = itertools.cycle(data)
except BaseException:
    print >> sys.stderr, "Single Iteration."

try:
    delay = float(args['--delay'])
    print >> sys.stderr, "Delay: %r" % delay
except BaseException:
    print >> sys.stderr, "Using default delay."

for line in data_iter:
    sys.stdout.write(line)
    sys.stdout.flush()
    time.sleep(delay)
