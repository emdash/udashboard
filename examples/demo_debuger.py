#! /bin/bash
#
# Run me from project root: ./examples/demo.sh

python scripts/simulate.py \
       --sin "RPM"                0   6500      \
       --sin "ECT"                100 230       \
       --sin "OIL_PRESSURE"       45  60        \
       --identity "SESSION_TIME" | python3 scripts/debugger.py examples/tach.img
