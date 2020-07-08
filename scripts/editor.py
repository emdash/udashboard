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
# - better feedback and reporting of vm errors
# - allow a plain define that's just a lookup
# - mouse click to set / update current point.
# - mouse drag to set / update current point
# - make points draggable with the mouse.
# - redefinition should be an error

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
import math
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

    # def split_right(self, pos):
    #     return self.from_top_left(self.northeast() - Point(, self.width - pos, self.height)

    def split_top(self, pos):
        return self.from_top_left(self.northwest(), self.width, pos)

    def split_bottom(self, pos):
        tl = self.northwest() + Point(0, pos)
        return self.from_top_left(tl, self.width, self.height - pos)

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

    def __init__(self, target, bounds, env=None, trace=False):
        self.stack = []
        self.lists = []
        self.env = env if env is not None else {}
        self.target = target
        self.trace = Logger("VM:")
        self.trace.enable = trace
        self.layout_stack = [bounds]

    def run(self, program):
        self.trace("PROG:", program, self.env)
        for (pc, token) in enumerate(program):
            self.trace("PC:", "%3d %9s" % (pc, token))
            self.execute(token)
            self.trace("STAK:", "L:", self.stack, "R:", self.env)

    def execute(self, token):
        if token == "[":
            self.trace("LIST")
            self.lists.append([])
            return
        elif token == "]":
            self.trace("LIST")
            if len(self.lists) > 1:
                nested = self.lists.pop()
                self.lists[-1].append(nested)
            elif len(self.lists) == 1:
                self.push(self.lists.pop())
            else:
                raise VMError("Mismatched ]")
            return
        elif token == "loop":
            self.trace("LOOP")
            # body, token are the tuples we push from ]
            body = self.pop()
            collection = self.pop()
            for value in collection:
                self.push(value)
                self.run(body)
            return
        elif self.lists:
            self.trace("LIST")
            self.lists[-1].append(token)
        elif token in self.opcodes:
            self.trace("OPCD")
            self.opcodes[token](self)
        elif token in self.env:
            self.trace("FUNC")
            self.run(self.env[token])
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
        print(self.pop())

    def debug(self):
        print(self.stack)

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
    }


class Cursor(object):

    """Represents an editable region of the document"""

    trace = Logger("Cursor:")

    def __init__(self, left, right, limit):
        self.trace("__init__:", left, right, limit)
        assert 0 <= left <= right <= limit
        self.left = left
        self.right = right
        self.length = right - left
        self.limit = limit

    def clamp(self, left, right, limit):
        length = right - left
        ret = (
            max(0, min(limit - length, left)),
            max(length, min(limit, right)))
        self.trace("clamp:", left, right, limit, length, ret)
        # clamping should never change length of selection.
        assert ret[1] - ret[0] == length
        return ret

    def shift(self, dist, limit=None):
        limit = limit if limit is not None else self.limit
        self.trace("shift:", dist, limit, self)
        left, right = self.clamp(
            self.left + dist,
            self.right + dist,
            limit)
        return Cursor(
            left,
            right,
            limit)

    def set_size(self, size, side, limit=None):
        limit = limit if limit else self.limit
        self.trace("set_size:", size, side, limit, self)

        assert size >= 0

        if side < 0:
            (left, right) = (self.left, self.left + size)
        else:
            (left, right) = (self.right - size, self.right)

        (left, right) = self.clamp(left, right, limit)
        return Cursor(left, right, limit)

    def delete(self):
        ntokens = max(0, min(self.limit, self.length))
        return Cursor(self.left, self.left, self.limit - ntokens)

    def at_end(self):
        return self.left == self.right == self.limit

    def at_start(self):
        return self.left == self.right == self.limit

    def __str__(self):
        return "Cursor(%d, %d, %d)" % (self.left, self.right, self.limit)
    def __repr__(self): return self.__str__()
    def __eq__(self, o):
        return (self.left, self.right, self.limit) == (o.left, o.right, o.limit)


class EditorState(object):

    """Immutable representation of complete document state."""

    trace = Logger("EditorState:")
    trace.enable = False

    def __init__(self, cursor, token, prog):
        self.trace("__init__:", cursor, token, prog)
        assert isinstance(cursor, Cursor)
        assert cursor.length <= len(prog)
        assert cursor.limit == len(prog)

        self.cursor = cursor
        # holds the token currently being edited.
        self.token = token
        # the entire program, so far.
        self.prog = prog

    def __str__(self):
        return "State(%r, %r, %r)" % (self.cursor, self.token, self.prog)

    def __repr__(self):
        return self.__str__()

    @classmethod
    def empty(cls):
        """Returns A blank document."""
        return EditorState(Cursor(0, 0, 0), '', [])

    def update(self, cursor=None, token=None, prog=None, completions=None):
        """Return a copy of self with given properties updated."""
        self.trace("update:", cursor, token, prog, completions)

        return EditorState(
            cursor if cursor is not None else self.cursor,
            token if token is not None else self.token,
            prog if prog is not None else self.prog
        )

    def push(self, char):
        """Append a character to the current token.
        """
        self.trace("push_char:", self, char)
        return EditorState(
            self.cursor,
            self.token + char,
            list(self.prog),
        )

    def pop(self):
        """Remove the last character from the current token.

        If the token is empty, if the selection is nonempty, deletes the
        selection. If the selection is empty, deletes the character
        behind the selection.
        """
        self.trace("pop_char:", self)

        if len(self.token) > 0:
            return EditorState(
                self.cursor,
                str(self.token[0:-1]),
                self.prog)
        else:
            return self.delete()

    def insert(self):
        """Commit the current token to the document, at the cursor location.

        Has no effect if token is empty.

        If the current cursor spreads across tokens, the effect is of
        replacing the selection with the current token. Otherwise the
        token is inserted.
        """
        self.trace("insert_token:", self)

        if not self.token:
            return self

        token = self.parse_token()

        # always remember to copy the program before mutating it.
        # in Rust we could enforce this automatically.
        prog = list(self.prog)

        if self.cursor.length > 0:
            del prog[self.cursor.left:self.cursor.right]
            next_ = str(token)
        else:
            next_ = ''
        prog.insert(self.cursor.left, token)
        return self.update(
            self.cursor.shift(1, limit=len(prog)), next_, prog)

    def delete(self):
        self.trace("delete:", self)
        """Remove tokens spanned by the cursor from the document.

        If cursor spans exactly one token, token is set to the deleted
        token. Otherwise it is cleared.

        Has no effect if program or cursor is empty.
        """

        if not self.cursor.length:
            return self.move(-1, False)

        if not len(self.prog):
            return self

        prog = list(self.prog)
        del prog[self.cursor.left:self.cursor.right]
        return self.update(self.cursor.delete(), prog=prog)

    def move(self, direction, shift):
        """Move the cursor forward or backward.
        Has no effect if the program length is 0.

        Only the sign of direction is considered, with negative
        meaning left.

        If shift is true, both ends of the cursor are shifted,
        preserving the length of the selection. If preserve is false,
        the selection is collapsed as follows:
           - if selection length is > 1 cell, collapses to one cell
           - if selection length is 1, collapses to empty cell
           - if selection length is 0, surrounds the next cell

        If shift is true, token is cleared.  If shift is false, token
        is set to the surrounded token, which may be empty.
        """
        self.trace("move:", self, direction, shift)

        # if we're already at the limit, this is a no-op
        if direction < 0:
            if 0 == self.cursor.left == self.cursor.right:
                return self
        else:
            if self.cursor.left == self.cursor.right == self.cursor.limit:
                return self

        if shift:
            cursor = self.cursor.shift(direction)
        elif self.cursor.length == 0:
            if direction < 0:
                cursor = self.cursor.shift(-1).set_size(1, direction)
            else:
                cursor = self.cursor.shift(1).set_size(1, direction)
        elif self.cursor.length == 1:
            cursor = self.cursor.set_size(0, direction)
        else:
            cursor = self.cursor.set_size(1, direction)

        if cursor.length == 1:
            token = str(self.prog[cursor.left])
        else:
            token = ''
        return self.update(cursor, token)

    def parse_token(self):
        self.trace("parse_token:", self)

        try:
            (x, y) = point_re.match(self.token).groups()[0::2]
            return Point(x, y)
        except:
            pass

        try:
            return int(self.token)
        except:
            pass

        try:
            return float(self.token)
        except:
            return self.token

    def insert_point(self, x, y):
        return self.update(token="(%g,%g)" % (x,y)).insert()

    def allowable(self, env):
        """Determine which tokens can be inserted at the given position."""

        # return value
        allowable = set()
        illegal = {}

        # Create a lightweight surface. It's not clear to me if this
        # allows things that an ImageSurface wouldn't. But it should
        # be faster to operate on this target surface, since it
        # doesn't need to rasterize.
        scratch_surface = cairo.RecordingSurface(
            cairo.Content.COLOR_ALPHA,
            cairo.Rectangle(0, 0, 1024, 768)
        )

        # Try inserting every possible opcode.
        for token in VM.opcodes:
            # Insert the opcode at the current editor position.
            prog = self.update(token=token).insert().prog

            try:
                # Create a temporary VM instance and run to completion.

                # XXX: If we could clone the context exactly, we could
                # avoid having to re-run the entire program to test
                # each opcode. We only really need to check the given
                # opcode against the final stack and final context
                # state, but since there's no way to copy the context,
                # this is the only way to be sure.
                #
                # Something to investigate is whether it's faster to
                # simply replay the recording surface, given that this
                # would avoid interpreter overhead. But honestly, I
                # care more that this gets the correct result without
                # having to write a lot of code.
                temp = VM(cairo.Context(scratch_surface), env)
                temp.run(prog)
                allowable.add(token)
            except BaseException as e:
                tb = sys.exc_info()[2]
                illegal[token] = e


        return (allowable, illegal)


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
        self.cr.translate(*self.center)
        self.cr.rectangle(self.x, self.y, self.width, self.height)
        # self.cr.clip()
        return self.bounds

    def __exit__(self, unused1, unused2, unused3):
        self.cr.restore()


class Editor(object):

    trace = Logger("Editor:")
    code_gutter_height = 20.5
    vm_gutter_width = 125.5
    token_length = 55.0

    def __init__(self, update_cb):
        self.state = EditorState.empty()
        self.char_map = {
            Gdk.KEY_BackSpace: self.delete,
            Gdk.KEY_space: self.insert,
            Gdk.KEY_Return: self.insert,
            Gdk.KEY_Left: lambda: self.move_cursor(-1),
            Gdk.KEY_Right: lambda: self.move_cursor(1)
        }
        self.update_cb = update_cb
        self.allowable = []
        # Annoyingly, we need to be realized to calculate the default env.
        # self.update_allowable()
        self.env = {}
        self.transform = None

    def insert(self):
        self.trace("insert", self.state)
        self.state = self.state.insert()
        self.update_allowable()

    def delete(self):
        self.trace("delete:", self.state)
        self.state = self.state.pop()
        self.update_allowable()

    def move_cursor(self, dist):
        self.trace("move:", self.state)
        self.state = self.state.move(dist, False)
        self.update_allowable()

    def update_allowable(self):
        self.allowable = list(self.state.allowable(self.env)[0])
        self.allowable.sort()

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
        rect = Rect(Point(0, 0), tw, th).inset(-2.5)
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

    def run(self, cr, env, origin, scale, window_size):
        self.trace("run:", self.state)

        window = Rect.from_top_left(Point(0, 0), window_size.x, window_size.y)

        content = window\
            .split_top(window.height - self.code_gutter_height)\
            .split_left(window.width - self.vm_gutter_width)

        code_gutter = window\
            .split_bottom(window.height - self.code_gutter_height)

        vm_gutter = Rect.from_top_left(
            content.northeast(),
            self.vm_gutter_width,
            content.height
        )

        # set default context state
        cr.set_source_rgb(0, 0, 0)
        cr.set_line_width(1.0)

        bounds = Rect(Point(0, 0), content.width / scale.x, content.height / scale.y)
        # cache environment, needed for computing allowable opcodes
        self.env = env

        with Subdivide(cr, content):
            cr.scale(scale.x, scale.y)

            # create a new vm instance with the window as the target.
            try:
                error = None
                vm = VM(cr, bounds, env, False)
                vm.run(self.state.prog)
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
            cr.move_to(*content.southwest())
            cr.rel_line_to(window.width, 0)
            cr.move_to(*vm_gutter.northwest())
            cr.line_to(*vm_gutter.southwest())
            cr.stroke()

        # draw the visible region of the bytecode.
        with Subdivide(cr, code_gutter) as bounds:
            cursor = self.state.cursor

            if cursor.length <= 1:
                selected = [self.state.token]
            else:
                selected = self.state.prog[cursor.left:cursor.right]

            with Save(cr):
                w = 0
                for token in selected:
                    (width, _) = self.token(cr, str(token), False)
                    cr.translate(width + 5.0, 0.0)
                    w += width + 10.0

            with Save(cr):
                cr.translate(-w / 2.0 - 5.0, 0.0)
                for token in reversed(self.state.prog[:cursor.left]):
                    (width, height) = self.token(cr, str(token))
                    cr.translate(-(width + 5.0), 0.0)

            with Save(cr):
                cr.translate(w / 2.0 + 5.0, 0.0)
                for token in self.state.prog[cursor.right:]:
                    (width, height) = self.token(cr, str(token))
                    cr.translate(width + 5.0, 0.0)

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

        with Subdivide(cr, content) as bounds:
            # show the current vm error, if any
            if error is not None:
                _, _, tw, _, _, _ = cr.text_extents(repr(error))
                cr.move_to(*bounds.southwest() + Point(5, -10))
                cr.show_text(repr(error))


    def handle_key_event(self, event):
        self.trace("handle_key_event:", self.state)
        self.handle_key(event.keyval)

    def handle_key(self, key):
        self.trace("enter: handle_key:", self.state, key)
        if key in self.char_map:
            self.char_map[key]()
        elif 0 <= key <= 255:
            self.trace("key:", chr(key))
            self.state = self.state.push(chr(key))
        else:
            print("unhandled:")
        self.trace("exit:  handle_key:", self.state)
        self.update_cb()

    def handle_button_press(self, event):
        (x, y) = self.inverse_transform.transform_point(event.x, event.y)
        self.state = self.state.insert_point(x, y)
        self.update_cb()

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

    def defenv(screen, origin):
        """Prepare the standard VM Environment."""
        return {
            "pi": [math.pi],
            "degrees": [2 * math.pi / 360.0, '*'],
        }

    def draw(widget, cr):
        # get window / screen geometry
        alloc = widget.get_allocation()
        screen = Point(float(alloc.width), float(alloc.height))
        origin = screen * 0.5
        scale = dpi(widget)

        # excute the program
        editor.run(cr, defenv(screen, origin), origin, scale, screen)

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

    editor = Editor(update)
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

def test():
    def case(c, cursor, token, prog):
        if isinstance(c, str):
            kv = ord(c)
        else:
            kv = c
        e.handle_key(kv)
        l, r, ll = cursor
        try:
            assert e.state.cursor == Cursor(l, r, ll)
            assert e.state.token == token
            assert e.state.prog == prog
        except AssertionError as err:
            print(e.state, "!=", cursor, token, prog)
            raise err

    assert Cursor(0, 0, 0) == Cursor(0, 0, 0)
    assert Cursor(0, 1, 1) == Cursor(0, 1, 1)
    assert Cursor(0, 0, 1) != Cursor(0, 0, 2)
    assert Cursor(1, 1, 1) == Cursor(1, 1, 1)

    e = Editor(lambda: None)
    case('f',               (0, 0, 0),  'f',    [])
    case('o',               (0, 0, 0),  'fo',   [])
    case('o',               (0, 0, 0),  'foo',  [])
    case(Gdk.KEY_space,     (1, 1, 1),  '',     ['foo'])
    case(Gdk.KEY_space,     (1, 1, 1),  '',     ['foo'])
    case(Gdk.KEY_Left,      (0, 1, 1),  'foo',  ['foo'])
    case(Gdk.KEY_Left,      (0, 0, 1),  '',     ['foo'])
    case(Gdk.KEY_Right,     (0, 1, 1),  'foo',  ['foo'])
    case(Gdk.KEY_Right,     (1, 1, 1),  '',     ['foo'])
    case('b',               (1, 1, 1),  'b',    ['foo'])
    case('a',               (1, 1, 1),  'ba',   ['foo'])
    case('r',               (1, 1, 1),  'bar',  ['foo'])
    case(Gdk.KEY_space,     (2, 2, 2),  '',     ['foo', 'bar'])
    case('0',               (2, 2, 2),  '0',    ['foo', 'bar'])
    case(Gdk.KEY_Return,    (3, 3, 3),  '',     ['foo', 'bar', 0])
    case(Gdk.KEY_Left,      (2, 3, 3),  '0',    ['foo', 'bar', 0])
    case(Gdk.KEY_Right,     (3, 3, 3),  '',     ['foo', 'bar', 0])
    case(Gdk.KEY_Left,      (2, 3, 3),  '0',    ['foo', 'bar', 0])
    case(Gdk.KEY_Left,      (2, 2, 3),  '',     ['foo', 'bar', 0])
    case(Gdk.KEY_Left,      (1, 2, 3),  'bar',  ['foo', 'bar', 0])
    case(Gdk.KEY_Left,      (1, 1, 3),  '',     ['foo', 'bar', 0])
    case(Gdk.KEY_Left,      (0, 1, 3),  'foo',  ['foo', 'bar', 0])
    case(Gdk.KEY_Left,      (0, 0, 3),  '',     ['foo', 'bar', 0])
    case(Gdk.KEY_Left,      (0, 0, 3),  '',     ['foo', 'bar', 0])
    case(Gdk.KEY_Right,     (0, 1, 3),  'foo',  ['foo', 'bar', 0])
    case(Gdk.KEY_Right,     (1, 1, 3),  '',     ['foo', 'bar', 0])
    case(Gdk.KEY_Right,     (1, 2, 3),  'bar',  ['foo', 'bar', 0])
    case(Gdk.KEY_Right,     (2, 2, 3),  '',     ['foo', 'bar', 0])
    case(Gdk.KEY_Right,     (2, 3, 3),  '0',    ['foo', 'bar', 0])
    case(Gdk.KEY_Right,     (3, 3, 3),  '',     ['foo', 'bar', 0])
    case(Gdk.KEY_Right,     (3, 3, 3),  '',     ['foo', 'bar', 0])
    case(Gdk.KEY_BackSpace, (2, 3, 3),  '0',    ['foo', 'bar', 0])
    case(Gdk.KEY_BackSpace, (2, 3, 3),  '',     ['foo', 'bar', 0])
    case(Gdk.KEY_BackSpace, (2, 2, 2),  '',     ['foo', 'bar'])
    case(Gdk.KEY_Left,      (1, 2, 2),  'bar',  ['foo', 'bar'])
    case(Gdk.KEY_Left,      (1, 1, 2),  '',     ['foo', 'bar'])
    case(Gdk.KEY_Right,     (1, 2, 2),  'bar',  ['foo', 'bar'])
    case(Gdk.KEY_Right,     (2, 2, 2),  '',     ['foo', 'bar'])
    case(Gdk.KEY_BackSpace, (1, 2, 2),  'bar',  ['foo', 'bar'])
    case(Gdk.KEY_BackSpace, (1, 2, 2),  'ba',   ['foo', 'bar'])
    case(Gdk.KEY_BackSpace, (1, 2, 2),  'b',    ['foo', 'bar'])
    case(Gdk.KEY_BackSpace, (1, 2, 2),  '',     ['foo', 'bar'])
    case(Gdk.KEY_BackSpace, (1, 1, 1),  '',     ['foo'])
    case(Gdk.KEY_BackSpace, (0, 1, 1),  'foo',  ['foo'])
    case(Gdk.KEY_BackSpace, (0, 1, 1),  'fo',   ['foo'])
    case(Gdk.KEY_BackSpace, (0, 1, 1),  'f',    ['foo'])
    case(Gdk.KEY_BackSpace, (0, 1, 1),  '',     ['foo'])
    case(Gdk.KEY_BackSpace, (0, 0, 0),  '',     [])

if __name__ == "__main__":
    import sys
    if len(sys.argv) >1 and sys.argv[1] == "test":
        Logger.enable = True
        test()
    elif len(sys.argv) > 1 and sys.argv[1] == "gui":
        import traceback
        print("GUI")
        Logger.enable = False
        gui()
    else:
        while True:
            print(handle_input(raw_input("> ")))

            continue
            env = {
                "screen": [200, 200, "point"],
                "origin": [0, 0, "point"]
            }
            vm = VM(None, env, trace=True)
            vm.run(prog)
