# uDashBoard: featherweight dashboard application.
#
# Copyright (C) 2019  Brandon Lewis
#
# This program is free software: you can redistribute it and/or
# modify it under the terms of the GNU Lesser General Public License
# as published by the Free Software Foundation, either version 3 of
# the License, or (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
# Lesser General Public License for more details.
#
# You should have received a copy of the GNU Lesser General Public
# License along with this program.  If not, see
# <https://www.gnu.org/licenses/>.

"""
Prototype Image Editor and Renderer

This is also the prototype virtual machine. It's to test the idea that
a model of directly editing the bytecode of a concatenated language.

The basic idea is as follows:
- Image data is stored as the bytecode for a stack-based vm.
- For reach frame, the screen is cleared, and the entire program
  re-run.
- All operations on the image are performed as updates to the
  bytecode, which is subsequently re-run.

"""

import gi
gi.require_version("Gtk", "3.0")
gi.require_foreign("cairo")
from gi.repository import GObject
from gi.repository import Gtk
from gi.repository import Gdk
import cairo
import math
import threading

class VMError(Exception): pass

class Point(object):

    def __init__(self, x, y):
        self.x = x
        self.y = y

    def __add__(self, o):
        return Point(self.x + o.x, self.y + o.y)

    def __sub__(self, o):
        return Point(self.x - o.x, self.y - o.y)

    def __mul__(self, s):
        return Point(self.x * s, self.y * s)

    def __rmul__(self, s):
        return Point(self.x * s, self.y * s)

    def __cmp__(self, o):
        return cmp((self.x, self.y), (o.x, o.y))

    def __repr__(self):
        return "(%g, %g)" % (self.x, self.y)

    def __len__(self):
        return math.sqrt(self.x ** 2 + self.y ** 2)

    def __iter__(self):
        yield self.x
        yield self.y


def frange(lower, upper, step):
    accum = lower
    while accum < upper:
        yield accum
        accum += step


class VM(object):

    def __init__(self, target, env={}, trace=False):
        self.stack = []
        self.lists = []
        self.env = env
        self.target = target
        self.trace = trace

    def run(self, program):
        if self.trace:
            print "PROG:", program
        for (pc, token) in enumerate(program):
            if self.trace:
                print "PC: %3d %9s" % (pc, token),
            self.execute(pc, token, program)
            if self.trace:
                print "STAK: %r L: %r " % (self.stack, self.env)

    def execute(self, pc, token, program):
        if token == "[":
            if self.trace:
                print " LIST ",
            self.lists.append([])
            return
        elif token == "]":
            if self.trace:
                print " LIST ",
            if len(self.lists) > 1:
                nested = self.lists.pop()
                self.lists[-1].append(nested)
            elif len(self.lists) == 1:
                self.push(self.lists.pop())
            else:
                raise VMError("Mismatched ]")
            return
        elif token == "loop":
            if self.trace:
                print " LOOP ",
            # body, token are the tuples we push from ]
            body = self.pop()
            collection = self.pop()
            for value in collection:
                self.push(value)
                self.run(body)
            return

        if self.lists:
            if self.trace:
                print " LIST ",
            self.lists[-1].append(self.parse(token))
        elif token in self.opcodes:
            if self.trace:
                print " OPCD ",
            self.opcodes[token](self)
        elif token in self.env:
            if self.trace:
                print " FUNC ",
            self.run(self.env[token])
        else:
            if self.trace:
                print " PUSH ",
            self.push(self.parse(token))

    def parse(self, token):
        try:
            return int(token)
        except:
            try:
                return float(token)
            except:
                return token

    def push(self, val):
        self.stack.insert(0, val)

    def peek(self, index=0):
        return self.stack[index]

    def poke(self, value, index=0):
        self.stack[index] = value

    def pop(self, index=0):
        try:
            return self.stack.pop(index)
        except:
            raise VMError("Stack underflow")

    # --- OPCODES
    def swap(self):
        temp = self.peek(0)
        self.poke(self.peek(1))
        self.poke(temp, 1)

    def drop(self): self.pop()
    def dup(self):  self.push(self.peek(0))
    def rel(self):  self.push(self.peek(self.pop()))
    def add(self):  self.push(self.pop() + self.pop())
    def sub(self):
        b = self.pop()
        a = self.pop()
        self.push(a - b)
    def mul(self):  self.push(self.pop() * self.pop())
    def div(self):  self.push(self.pop() * self.pop())
    def mod(self):  self.push(self.pop() % self.pop())
    def max(self):  self.push(max(self.pop(), self.pop()))
    def min(self):  self.push(min(self.pop(), self.pop()))

    def define(self):
        body = self.pop()
        name = self.pop()
        self.env[name] = body

    def load(self):
        self.push(self.env[self.pop()])

    def range(self):
        step = self.pop()
        upper = self.pop()
        lower = self.pop()
        self.push(frange(lower, upper, step))

    def unquote(self):
        self.run(self.pop())

    def point(self):
        y = self.pop()
        x = self.pop()
        self.push(Point(x, y))

    def unpack(self):
        pt = self.pop()
        self.push(pt.y)
        self.push(pt.x)

    def len(self):
        self.push(len(self.pop()))

    def circle(self):
        radius = self.pop()
        (x, y) = self.pop()
        self.target.arc(x, y, radius, 0, 2 * math.pi)

    def rectangle(self):
        h = self.pop()
        w = self.pop()
        (x, y) = self.pop()
        self.target.rectangle(x - w * 0.5, y - h * 0.5, w, h)

    def moveto(self):
        (x, y) = self.pop()
        self.target.move_to(x, y)

    def lineto(self):
        (x, y) = self.pop()
        self.target.line_to(x, y)

    def stroke(self):
        self.target.stroke()

    def fill(self):
        self.target.fill()

    def save(self):
        self.target.save()

    def restore(self):
        self.target.restore()

    def translate(self):
        (x, y) = self.pop()
        self.target.translate(x, y)

    def rotate(self):
        self.target.rotate(self.pop())

    def scale(self):
        y = self.pop()
        x = self.pop()
        self.target.scale(x, y)

    def paint(self):
        self.target.paint()

    def disp(self):
        print self.pop()

    def debug(self):
        print self.stack

    opcodes = {
        "drop":      drop,
        "dup":       dup,
        "rel":       rel,
        "swap":      swap,
        "+":         add,
        "-":         sub,
        "*":         mul,
        "/":         div,
        "%":         mod,
        "min":       min,
        "max":       max,
        "define":    define,
        "load":      load,
        "range":     range,
        "unquote":   unquote,
        "point":     point,
        "unpack":    unpack,
        "len":       len,
        "rectangle": rectangle,
        "moveto":    moveto,
        "lineto":    lineto,
        "circle":    circle,
        "fill":      fill,
        "stroke":    stroke,
        "save":      save,
        "restore":   restore,
        "translate": translate,
        "scale":     scale,
        "rotate":    rotate,
        "paint":     paint,
        ".":         disp,
        "!":         debug
    }

prog = []
env = {}
window = Gtk.Window()
da = Gtk.DrawingArea()
da.set_events(Gdk.EventMask.ALL_EVENTS_MASK)
window.add(da)
window.show_all()
window.connect("destroy", Gtk.main_quit)
def update(widget, cr):
    global env
    alloc = widget.get_allocation()
    ox, oy = (alloc.width / 2, alloc.height / 2)
    env = {
        "screen": [alloc.width, alloc.height, "point"],
        "origin": [ox, oy, "point"],
        "pi": [ str(math.pi) ]
    }
    vm = VM(cr, env)
    cr.save()
    cr.set_source_rgb(0, 0, 0)
    cr.translate(ox, oy)
    vm.run(prog)
    cr.restore()
    cr.move_to(0, alloc.height - 5)
    cr.show_text(repr(vm.stack))
da.connect('draw', update)

def printargs(*args):
    print args

def insert_point(unused, event):
    prog.extend([event.x, event.y, "point"])
    da.queue_draw()

da.connect('button-press-event', insert_point)


def handle_input():
    global prog
    global env
    inp = raw_input("> ")
    if inp == ":?":
        for token in prog:
            print token
    elif inp == ":clr":
        prog = []
        env = {}
    elif inp == ":undo":
        prog.pop()
    elif inp == ":quit":
        exit(0)
    elif inp == ":save":
        f = open("image.dat", "w")
        for token in prog:
            print token
            print >> f, token
        f.close()
    elif inp == ":load":
        prog = [l.strip() for l in open("image.dat", "r")]
    elif inp.startswith(":set"):
        cmd, name, val = inp.split()
        env[name] = [val]
        print env
    else:
        prog.extend(inp.split())
        print prog

def mainloop():
    try:
        while True:
            handle_input()
            da.queue_draw()
    finally:
        Gtk.main_quit()



if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1 and sys.argv[1] == "gui":
        repl = threading.Thread(target=mainloop)
        repl.daemon = True
        repl.start()
        Gtk.main()
    else:
        while True:
            handle_input()
            env = {
                "screen": [200, 200, "point"],
                "origin": [0, 0, "point"]
            }
            vm = VM(None, env, trace=True)
            vm.run(prog)
