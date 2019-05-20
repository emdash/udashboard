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


class VM(object):

    def __init__(self, target, env={}):
        self.stack = []
        self.env = env
        self.list = []
        self.target = target

    def run(self, program):
        for token in program:
            self.execute(token)

    def execute(self, token):
        if token == "[":
            self.list.append([])
            return
        elif token == "]":
            lst = self.list.pop()
            if self.list:
                self.list[-1].append(lst)
            else:
                self.push(lst)
            return

        if self.list:
            self.list[-1].append(token)
            return

        if token in self.opcodes:
            self.opcodes[token](self)
        elif token in self.env:
            self.run(self.env[token])
        else:
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
    def sub(self):  self.push(self.pop() - self.pop())
    def mul(self):  self.push(self.pop() * self.pop())
    def div(self):  self.push(self.pop() * self.pop())
    def mod(self):  self.push(self.pop() % self.pop())
    def max(self):  self.push(max(self.pop(), self.pop()))
    def min(self):  self.push(min(self.pop(), self.pop()))

    def loop(self):
        body = self.pop()
        collection = self.pop()
        for value in collection:
            self.push(value)
            self.run(body)

    def define(self):
        body = self.pop()
        name = self.pop()
        self.env[name] = body

    def load(self):
        self.push(self.env[self.pop()])

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
        x, y, w, h = self.pop()
        self.target.rectangle(x, y, w, h)

    def moveto(self):
        x, y = self.pop()
        self.target.move_to(x, y)

    def lineto(self):
        x, y = self.pop()
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
        y = self.pop()
        x = self.pop()
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
        "loop":      loop,
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
    env.update({
        "screen": [alloc.width, alloc.height, "point"],
        "center": ["0.5", "screen", "*"],
    })
    vm = VM(cr, env)
    cr.set_source_rgb(0, 0, 0)
    cr.save()
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


def mainloop():
    global prog
    global env
    try:
        while True:
            inp = raw_input("> ")
            if inp == ":?":
                for token in prog:
                    print token
            elif inp == ":clr":
                prog = []
            elif inp == ":undo":
                prog.pop()
            elif inp == ":quit":
                Gtk.main_quit()
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
            da.queue_draw()
    finally:
        Gtk.main_quit()

repl = threading.Thread(target=mainloop)
repl.daemon = True
repl.start()
Gtk.main()
