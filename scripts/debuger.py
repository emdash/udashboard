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
Prototype Image Editor and Renderer.

This is also the prototype virtual machine. It's a testbed for the
whole concept of self-contained dynamic vector graphics.

The basic idea is as follows:
- Image data is stored as the bytecode for a stack-based vm.
- For reach frame, the screen is cleared, and the entire program
  re-run.
- All operations on the image are performed as updates to the
  bytecode, which is subsequently re-run.

"""

# TODO:
# - type checking in the vm
  # number (int or float)
  # range
  # point
  # enum(*values)
  # list(type)
  # tuple(*types)
# - mouse click to set / update current point.
# - mouse drag to set / update current point
# - make points draggable with the mouse.

# MISSING FEATURES
# - save and load files (define header format)
#   - define top-level format and meta-data
#   - physicial size and editor should render the
#   - version string
# - click-and drag to update control points
# - click-and-drag for affine transforms.
# - show current pattern
# - color chooser
# - gradients
# - keyboard shortcuts
#   bounds
# - font selection
# - text size selection
# - text strings (escaping)
# - live data input (from stdin)
# - composition of smaller images
# - zoom and pan

# OPEN ISSUES
# - is direct editing of bytecode the right model?
# - explore modal editing
  # - vim-style command mode
# - what's the right model for working with variables?
# - what's the right model for working with functions?
# - what's the right model for working with expressions

from __future__ import print_function

import gi
gi.require_version("Gtk", "3.0")
gi.require_foreign("cairo")
from gi.repository import GObject
from gi.repository import Gtk
from gi.repository import Gdk
import cairo
import json
import math
import pyinotify
import threading
from queue import Queue
import re
import sys
import time

class VMError(Exception): pass
class LexError(Exception): pass


point_re = re.compile(r"^\((-?\d+(\.\d+)?),(-?\d+(\.\d+)?)\)$")


class Logger(object):

    """Simple but featurfule logger.

    Add class-level instance for any class that needs logging
    functionality. Implements __call__ so it can be called like a
    regular method. Also works as a context manager.

    """

    enable = False

    def __init__(self, name):
        self.name = name

    def __call__(self, prefix, *args):
        """Prints a log message."""

        if self.enable:
            msg = ("%s %s " %
               (self.name, prefix) +
                " ".join((repr(arg) for arg in args)))
            print(msg, file=sys.stderr)
        else:
            return self

    def trace(self, *args):
        if self.enable:
            return self.Tracer(self, args)
        else:
            return self

    def __enter__(self, *args):
        """Dummy context manager interface when logging is disabled"""
        pass

    def __exit__(self, *args):
        """Dummy context manager interface when logging is disabled"""
        pass

    class Tracer(object):
        """Context manager logging."""
        def __init__(self, logger, args):
            self.logger = logger
            self.args = args

        def __enter__(self, *unused):
            self.logger("enter:")

        def __exit__(self, *unused):
            self.logger("exit:")


class Point(object):

    """Reasonably terse 2D Point class."""

    def __init__(self, x, y): self.x = float(x) ; self.y = float(y)
    def __len__(self):        return math.sqrt(self.x ** 2 + self.y ** 2)
    def __eq__(self, o):
        return isinstance(o, Point) and (self.x, self.y) == (o.x, o.y)
    def __repr__(self):       return "(%g,%g)" % (self.x, self.y)
    def __iter__(self):       yield  self.x ; yield self.y
    def __hash__(self):       return hash((self.x, self.y))
    def __bool__(self):       return False

    def binop(func):
        def impl(self, x):
            o = x if isinstance(x, Point) else Point(x, x)
            return Point(func(self.x, o.x), func(self.y, o.y))
        return impl

    __add__  = binop(lambda a, b: a + b)
    __sub__  = binop(lambda a, b: a - b)
    __mul__  = binop(lambda a, b: a * b)
    __rsub__ = binop(lambda a, b: b - a)
    __rmul__ = binop(lambda a, b: b * a)
    __truediv__  = binop(lambda a, b: a / b)
    __rtruediv__ = binop(lambda a, b: b / a)



class Rect(object):

    """Rectangle operations for layout."""

    def __init__(self, center, width, height):
        self.center = center
        self.width = width
        self.height = height

    @classmethod
    def from_top_left(self, top_left, width, height):
        return Rect(
            Point(top_left.x + width * 0.5, top_left.y + height * 0.5),
            width, height
        )

    def __repr__(self):
        return "(%s, %g, %g)" % (self.center, self.width, self.height)

    def north(self):
        return self.center + Point(0, -0.5 * self.height)

    def south(self):
        return self.center + Point(0, 0.5 * self.height)

    def east(self):
        return self.center + Point(0.5 * self.width, 0)

    def west(self):
        return self.center + Point(-0.5 * self.width, 0)

    def northwest(self):
        return self.center + Point(-0.5 * self.width, -0.5 * self.height)

    def northeast(self):
        return self.center + Point(0.5 * self.width, -0.5 * self.height)

    def southeast(self):
        return self.center + Point(0.5 * self.width, 0.5 * self.height)

    def southwest(self):
        return self.center + Point(-0.5 * self.width, 0.5 * self.height)

    def inset(self, size):
        amount = size * 2
        return Rect(self.center, self.width - amount, self.height - amount)

    def split_left(self, pos):
        return self.from_top_left(self.northwest(), pos, self.height)

    def split_right(self, pos):
        tl = self.northwest() + Point(pos, 0)
        return self.from_top_left(tl, self.width - pos, self.height)

    def split_top(self, pos):
        return self.from_top_left(self.northwest(), self.width, pos)

    def split_bottom(self, pos):
        tl = self.northwest() + Point(0, pos)
        return self.from_top_left(tl, self.width, self.height - pos)

    def split_vertical(self, pos):
        return (self.split_left(pos), self.split_right(pos))

    def split_horizontal(self, pos):
        return (self.split_top(pos), self.split_bottom(pos))

    def radius(self):
        return min(self.width, self.height) * 0.5


def frange(lower, upper, step):
    """Like xrange, but for floats."""
    accum = lower
    while accum < upper:
        yield accum
        accum += step


class VM(object):
    """Executes bytecode on the given cairo context."""

    joins = {
        "bevel": cairo.LINE_JOIN_BEVEL,
        "miter": cairo.LINE_JOIN_MITER,
        "round": cairo.LINE_JOIN_ROUND
    }
    caps = {
        "butt": cairo.LINE_CAP_BUTT,
        "round": cairo.LINE_CAP_ROUND,
        "square": cairo.LINE_CAP_SQUARE
    }

    def __init__(self, target, bounds, trace=False):
        self.stack = []
        self.target = target
        self.trace = Logger("VM:")
        self.trace.enable = trace
        self.layout_stack = [bounds]
        self.debug_output = []

    def run(self, program, target, env):
        local = {}
        self.trace("PROG:", program)
        for token in program[target]:
            self.execute(token, program, local, env)

    def execute(self, token, program, local, env):
        self.trace("EXEC:", token)
        if token == "loop":
            self.trace("LOOP")
            body = self.pop()
            collection = self.pop()
            for item in collection:
                self.push(item)
                self.run(program, body, env)
        elif token == "define":
            symbol = self.pop()
            value = self.pop()
            if (symbol in self.opcodes
                or symbol in env
                or symbol in program
                or symbol in local
            ):
                raise VMError("Redefinition of symbol %s" % token)
            else:
                local[symbol] = value
        elif token in self.opcodes:
            self.trace("OPCD")
            self.opcodes[token](self)
        elif token in local:
            self.trace("LOCAL")
            self.push(local[token])
        elif token in env:
            self.trace("ENV")
            self.push(env[token])
        elif token in program:
            self.trace("FUNC")
            self.run(program, token, env)
        elif isinstance(token, str) and token.startswith(":"):
            self.push(token[1:])
        else:
            self.trace("PUSH")
            self.push(token)

    def push(self, val):
        self.stack.append(val)

    def peek(self, index=0):
        return self.stack[-index]

    def poke(self, value, index=0):
        self.stack[index] = value

    def pop(self):
        if self.stack:
            return self.stack.pop()
        else:
            raise VMError("Stack underflow")

    # --- OPCODES

    def swap(self):
        a = self.pop()
        b = self.pop()
        self.push(a)
        self.push(b)

    def drop(self): self.pop()
    def dup(self):  self.push(self.peek(0))
    def rel(self):  self.push(self.peek(self.pop()))
    def add(self):  self.push(self.pop() + self.pop())
    def sub(self):
        b = self.pop()
        a = self.pop()
        self.push(a - b)
    def div(self):
        b = self.pop()
        a = self.pop()
        self.push(a / b)
    def mul(self):  self.push(self.pop() * self.pop())
    def mod(self):
        b = self.pop()
        a = self.pop()
        self.push(a % b)
    def max(self):  self.push(max(self.pop(), self.pop()))
    def min(self):  self.push(min(self.pop(), self.pop()))

    def define(self):
        name = self.pop()
        body = self.pop()
        assert isinstance(body, list)
        assert isinstance(name, str)
        self.env[name] = body

    def load(self):
        self.name = self.pop()
        assert isinstance(name, str)
        self.push(self.env[self.pop()])

    def range(self):
        step = self.pop()
        upper = self.pop()
        lower = self.pop()
        self.push(list(frange(lower, upper, step)))

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

    def rgb(self):
        b = self.pop()
        g = self.pop()
        r = self.pop()
        self.push(cairo.SolidPattern(r, g, b))

    def rgba(self):
        a = self.pop()
        b = self.pop()
        g = self.pop()
        r = self.pop()
        self.push(cairo.SolidPattern(r, g, b, a))

    def circle(self):
        radius = self.pop()
        self.target.arc(0, 0, radius, 0, 2 * math.pi)

    def arc(self):
        end = self.pop()
        start = self.pop()
        radius = self.pop()
        self.target.arc(0, 0, radius, start, end)

    def rectangle(self):
        h = self.pop()
        w = self.pop()
        self.target.rectangle(w * -0.5, h * -0.5, w, h)

    def moveto(self):
        (x, y) = self.pop()
        self.target.move_to(x, y)

    def lineto(self):
        (x, y) = self.pop()
        self.target.line_to(x, y)

    def curveto(self):
        (x3, y3) = self.pop()
        (x2, y2) = self.pop()
        (x1, y1) = self.pop()
        self.target.curve_to(x1, y1, x2, y2, x3, y3)

    def close(self):
        self.target.close_path()

    def new(self):
        self.target.new_path()

    def subpath(self):
        self.target.new_sub_path()

    def source(self):
        self.target.set_source(self.pop())

    def linewidth(self):
        self.target.set_line_width(self.pop())

    def linejoin(self):
        self.target.set_line_join(self.joins.get(self.pop()))

    def linecap(self):
        self.target.set_line_cap(self.caps.get(self.pop()))

    def stroke(self):
        self.target.stroke()

    def fill(self):
        self.target.fill()

    def clip(self):
        self.target.clip()

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
        self.debug_output.append(self.peek(0))

    def debug(self):
        self.debug_output.append(list(self.stack))

    def bounds(self):
        self.push(self.layout_stack[-1])

    def center(self):
        self.push(self.layout_stack[-1].center)

    def top(self):
        self.push(self.layout_stack[-1].north().y)

    def bottom(self):
        self.push(self.layout_stack[-1].south().y)

    def left(self):
        self.push(self.layout_stack[-1].west().x)

    def right(self):
        self.push(self.layout_stack[-1].east().x)

    def width(self):
        self.push(self.layout_stack[-1].width)

    def height(self):
        self.push(self.layout_stack[-1].height)

    def north(self):
        self.push(self.layout_stack[-1].north())

    def south(self):
        self.push(self.layout_stack[-1].south())

    def east(self):
        self.push(self.layout_stack[-1].east())

    def west(self):
        self.push(self.layout_stack[-1].west())

    def northeast(self):
        self.push(self.layout_stack[-1].northeast())

    def southeast(self):
        self.push(self.layout_stack[-1].southeast())

    def northwest(self):
        self.push(self.layout_stack[-1].northwest())

    def southwest(self):
        self.push(self.layout_stack[-1].southwest())

    def inset(self):
        size = self.pop()
        self.layout_stack.append(self.layout_stack[-1].inset(size))

    def radius(self):
        self.push(max(0, self.layout_stack[-1].radius()))

    def pop_bounds(self):
        self.layout_stack.pop(-1)

    def sin(self):
        self.push(math.sin(self.pop()))

    def cos(self):
        self.push(math.cos(self.pop()))

    def abs(self):
        self.push(math.abs(self.pop()))

    def time(self):
        self.push(time.time())

    def pi(self):
        self.push(math.pi)


    ## end of opcodes

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
        "abs":       abs,
        "sin":       sin,
        "cos":       cos,
        "define":    define,
        "load":      load,
        "range":     range,
        "unquote":   unquote,
        "point":     point,
        "unpack":    unpack,
        "len":       len,
        "rgb":       rgb,
        "rgba":      rgba,
        "rectangle": rectangle,
        "moveto":    moveto,
        "lineto":    lineto,
        "curveto":   curveto,
        "circle":    circle,
        "arc":       arc,
        "fill":      fill,
        "clip":      clip,
        "close":     close,
        "new":       new,
        "subpath":   subpath,
        "source":    source,
        "linewidth": linewidth,
        "linejoin":  linejoin,
        "linecap":   linecap,
        "stroke":    stroke,
        "save":      save,
        "restore":   restore,
        "translate": translate,
        "scale":     scale,
        "rotate":    rotate,
        "paint":     paint,
        ".":         disp,
        "!":         debug,
        "bounds":    bounds,
        "top":       top,
        "bottom":    bottom,
        "left":      left,
        "right":     right,
        "width":     width,
        "height":    height,
        "center":    center,
        "north":     north,
        "south":     south,
        "east":      east,
        "west":      west,
        "northeast": northeast,
        "southeast": southeast,
        "northwest": northwest,
        "southwest": southwest,
        "inset":     inset,
        "radius":    radius,
        "pop":       pop_bounds,
        "time":      time,
        "pi":        pi
    }


def compile(prog):
    labels = {'main': []}
    cur_label = labels['main']
    cur_list = None
    lists = []

    def parse(token):
        try:
            return int(token)
        except:
            try:
                return float(token)
            except:
                return token

    for token in prog:
        if token.endswith(":"):
            label = token[:-1]
            if label in labels:
                raise VMError("Redefinition of %s" % label)
            else:
                cur_label = labels[label] = []
        elif token == "[":
            lists.append([])
        elif token == "]":
            cur_label.append(lists.pop())
        else:
            if lists:
                lists[-1].append(parse(token))
            else:
                cur_label.append(parse(token))

    return labels


class Save(object):

    def __init__(self, cr):
        self.cr = cr

    def __enter__(self):
        self.cr.save()

    def __exit__(self, unused1, unused2, unused3):
        self.cr.restore()


class Subdivide(object):

    def __init__(self, cr, bounds):
        self.cr = cr
        self.center = bounds.center
        self.bounds = Rect(Point(0, 0), bounds.width, bounds.height)
        (self.x, self.y) = bounds.northwest()
        self.width = bounds.width
        self.height = bounds.height

    def __enter__(self):
        self.cr.save()
        self.cr.rectangle(self.x, self.y, self.width, self.height)
        self.cr.clip()
        self.cr.translate(*self.center)
        return self.bounds

    def __exit__(self, unused1, unused2, unused3):
        self.cr.restore()


class EditorState(object):

    def __init__(self, path):
        self.path = path
        self.prog = None
        self.load()

    def load(self):
        prog = []
        for line in open(self.path, "r"):
            if not line.strip().startswith("#"):
                for token in line.split():
                    prog.append(token.strip())

        self.prog = compile(prog)
        print(self.prog)



class Editor(object):

    trace = Logger("Editor:")
    status_bar_height = 20.5
    vm_gutter_width = 125.5
    code_gutter_width = 125.5
    token_length = 55.0

    def __init__(self, reader):
        self.state = EditorState(sys.argv[1])
        self.allowable = []
        self.transform = None
        self.reader = reader
        self.reader.start()

    def text(self, cr, text):
        """Draw text centered at (0, 0)"""
        _, _, tw, th, _, _ = cr.text_extents(text)
        with Save(cr):
            cr.move_to(-tw / 2, th / 2)
            cr.show_text(text)
            return (tw, th)

    def rect(self, cr, rect):
        """Place the given rect into the path"""
        with Save(cr):
            (x, y) = rect.northwest()
            cr.rectangle(x, y, rect.width, rect.height)

    def token(self, cr, token, fill=True):
        _, _, tw, _, _, _ = cr.text_extents(token)
        th = 10
        rect = Rect(Point(0, 0), self.code_gutter_width - 5.0, th + 5.0)
        with Save(cr):
            self.rect(cr, rect)
            cr.set_source_rgb(0.5, 0.5, 0.5)
            if fill:
                cr.fill()
            else:
                cr.stroke()
            cr.set_source_rgb(0.0, 0.0, 0.0)
            self.text(cr, token)
        return (rect.width, rect.height)

    def run(self, cr, origin, scale, window_size):
        self.trace("run:", self.state)

        window = Rect.from_top_left(Point(0, 0), window_size.x, window_size.y)

        (remainder, status_bar) = window\
            .split_horizontal(window.height - self.status_bar_height)

        (remainder, vm_gutter) = remainder\
            .split_vertical(window.width - self.vm_gutter_width)

        (code_gutter, content) = remainder\
            .split_vertical(self.code_gutter_width)

        # set default context state
        cr.set_source_rgb(0, 0, 0)
        cr.set_line_width(1.0)

        bounds = Rect(Point(0, 0), content.width / scale.x, content.height / scale.y)

        with Subdivide(cr, content):
            cr.scale(scale.x, scale.y)

            # create a new vm instance with the window as the target.
            try:
                error = None
                vm = VM(cr, bounds, False)
                vm.run(self.state.prog, 'main', self.reader.env)
            except Exception as e:
                error = e

            self.transform = cr.get_matrix()
            self.inverse_transform = cr.get_matrix()
            self.inverse_transform.invert()

            # save the current point
            x, y = cr.get_current_point()

        with Save(cr):
            # stroke any residual path for feedback
            cr.set_source_rgb(1.0, 1.0, 1.0)
            cr.set_operator(cairo.OPERATOR_DIFFERENCE)
            cr.set_line_width(0.1)
            cr.stroke()

        with Save(cr):
            # draw the current point.
            x, y = self.transform.transform_point(x, y)
            cr.translate(x, y)
            cr.move_to(-5, 0)
            cr.line_to(5, 0)
            cr.move_to(0, -5)
            cr.line_to(0, 5)
            cr.stroke()

            # show any residual points on stack
            for item in vm.stack:
                if isinstance(item, Point):
                    (x, y) = self.transform.transform_point(item.x, item.y)
                    cr.arc(x, y, 0.5, 0, math.pi * 2)
                    cr.fill()

            # draw top two numbers on stack
            stack_nums = [
                i for i in reversed(vm.stack)
                if (isinstance(i, int) or isinstance(i, float))
            ] if vm.stack else []

            if len(stack_nums) == 1:
                cr.arc(0, 0, 0, math.pi * 2, stack_nums[0])
                cr.stroke()
            elif len(stack_nums) == 2:
                # XXX: fixme 
                # cr.move_to(-width / 2, stack_nums[0])
                # cr.line_to(width / 2,  stack_nums[0])
                # cr.stroke()
                # cr.move_to(stack_nums[1], -height / 2)
                # cr.line_to(stack_nums[1],  height / 2)
                # cr.stroke()
                pass

        # draw gutters around UI
        with Save(cr):
            cr.set_line_width(1.0)
            cr.move_to(*code_gutter.southwest())
            cr.rel_line_to(window.width, 0)
            cr.move_to(*vm_gutter.northwest())
            cr.line_to(*vm_gutter.southwest())
            cr.move_to(*code_gutter.southeast())
            cr.rel_line_to(0, -code_gutter.height)
            cr.stroke()

        # # draw the visible region of the bytecode.
        # with Subdivide(cr, code_gutter) as bounds:

        #     with Save(cr):
        #         h = 0
        #         for token in selected:
        #             _, height = self.token(cr, str(token), False)
        #             cr.translate(0, height + 5)
        #             h += height + 10

        #     with Save(cr):
        #         cr.translate(0.0, -h / 2.0 - 5.0)
        #         for token in reversed(self.state.prog[:cursor.left]):
        #             _, height = self.token(cr, str(token))
        #             cr.translate(0.0, -(height + 5))

        #     with Save(cr):
        #         cr.translate(0.0, h / 2.0 + 5.0)
        #         for token in self.state.prog[cursor.right:]:
        #             _, height = self.token(cr, str(token))
        #             cr.translate(0.0, height + 5)

        # with Subdivide(cr, window) as bounds:
        #     # draw the allowed commands for the current context
        #     cr.translate(*(bounds.northwest() + Point(10, 10)))
        #     for item in self.allowable:
        #         (w, h) = self.token(cr, str(item))
        #         cr.translate(w + 5.0, 0.0)

        with Subdivide(cr, vm_gutter) as bounds:
            cr.translate(*bounds.south())
            for item in reversed(vm.stack):
                cr.translate(0, -10)
                self.text(cr, repr(item))

        with Subdivide(cr, vm_gutter) as bounds:
            cr.translate(*bounds.northwest() + Point(0, 10))
            for item in sorted(self.reader.env):
                cr.move_to(0, 0)
                cr.show_text("%s: %r" % (item, self.reader.env[item]))
                cr.translate(0, 10)

        with Subdivide(cr, content) as bounds:
            # show the current vm error, if any
            if error is not None:
                _, _, tw, _, _, _ = cr.text_extents(repr(error))
                cr.move_to(*bounds.southwest() + Point(5, -10))
                cr.show_text(repr(error))

        with Subdivide(cr, status_bar) as bounds:
            cr.move_to(*bounds.west())
            cr.show_text(repr(vm.debug_output))


    def handle_key_event(self, event):
        self.trace("handle_key_event:", self.state)
        self.state.load()

    def handle_button_press(self, event):
        self.state.load()

    def handle_cmd(self, cmd):
        """Process the given string as a command."""
        if cmd == ":clr":
            self.clear()
        elif cmd == ":undo":
            pass
        elif cmd == ":quit":
            exit(0)
        elif cmd == ":save":
            f = open("image.dat", "w")
            for token in prog:
                print(token, file=f)
            f.close()
        elif cmd == ":load":
            self.prog = [l.strip() for l in open("image.dat", "r")]
        elif cmd.startswith(":set"):
            _, name, val = cmd.split()
            env[name] = [val]
            print(env)
        else:
            print("unrecognized cmd", cmd)


class ReaderThread(threading.Thread):

    env = {}
    daemon = True

    def run(self):
        while True:
            self.env = json.loads(sys.stdin.readline())


def notify_thread(editor):

    def modified(*unused, **unused2):
        GObject.idle_add(editor.state.load)

    wm = pyinotify.WatchManager()
    wm.add_watch(sys.argv[1], pyinotify.IN_MODIFY)
    notifier = pyinotify.ThreadedNotifier(wm, modified)
    notifier.start()


def gui():
    def dpi(widget):
        """Return the dpi of the current monitor as a Point."""
        s = widget.get_screen()
        m = s.get_monitor_at_window(window.get_window())
        geom = s.get_monitor_geometry(m)
        mm = Point(s.get_monitor_width_mm(m),
                   s.get_monitor_height_mm(m))
        size = Point(float(geom.width), float(geom.height))
        return size / mm

    def draw(widget, cr):
        # get window / screen geometry
        alloc = widget.get_allocation()
        screen = Point(float(alloc.width), float(alloc.height))
        origin = screen * 0.5
        scale = dpi(widget)

        # excute the program
        editor.run(cr, origin, scale, screen)

    def key_press(widget, event):
        editor.handle_key_event(event)

    def button_press(widget, event):
        editor.handle_button_press(event)

    def update():
        try:
            da.queue_draw()
        finally:
            return True


    GObject.timeout_add(25, update)

    editor = Editor(ReaderThread())
    notify_thread(editor)

    window = Gtk.Window()
    window.set_size_request(640, 480)
    da = Gtk.DrawingArea()
    da.set_events(Gdk.EventMask.ALL_EVENTS_MASK)
    window.add(da)
    window.show_all()
    window.connect("destroy", Gtk.main_quit)
    da.connect('draw', draw)
    window.connect('key-press-event', key_press)
    window.connect('button-press-event', button_press)
    Gtk.main()

if __name__ == "__main__":
    import traceback
    print("GUI")
    Logger.enable = False
    gui()
