# 5/19/2019 Editor Design

Consider a simple PostScript like stack language which maps onto the
cairo api:

  - Has scalars: `5.0`, `1`, `"foo"`
  - Has points: `(3, 0)`, `(1.5, 10)`
  - Has patterns: `#33ffcc`, `radial(...)`
  - Has operations: `moveto`, `lineto`, `setsource`, `fill`, `stroke`.

Example 1: draw a filled circle with 50 pixel radius

  `(0, 0) 50 ellipse #ccffcc setsource fill`

Example 2: stroke an open path:
```
  (0, 0) moveto
  (0, 5) lineto
  (5, 5) lineto
  0.5 setlinewidth
  'black
  stroke
```

This follows a "noun-verb" pattern, which we can exploit in a
gui. Looked at like this, the document *is* the sequence of
commands. This is an easy format to work with, and rendering is pretty
straight-forward. The operations all manipulate the stack in a uniform
way, and could either be builtin or user-defined.

There's a correspondence with mouse interaction: We could map UI
gestures to sequences of tokens:

  - mouse click on canvas  -> $cur_mouse_pos
  - click-and-drag         -> $mouse_down_pos $drag_delta
  - click "New Path"       -> moveto
  - press ESC              -> close_path
  - click "Line"           -> lineto
  - click "Arc"            -> arcto
  - click "Spline"         -> splineto
  - click "Fill"           -> fill_preserve
  - click "Stroke"         -> stroke_preserve
  - click "Set line width" -> setlinewidth
  - shift + click "Fill"   -> fill
  - shift + click "Stroke" -> stroke

Now we have a quick-and-dirty way to implement a graphical editor. We
can bind accelerator keys to these commands for a combo keyboard-mouse
workflow that's pretty efficient.

The mouse operations correspond to the text representation, so we can
show the two side-by-side and ensure they stay in sync.

The document is a sequence of commands that produces an image, and
this is the actual format that we save to disk. The primary workflow
is to add to the image by appending commands to the document via the
mouse. But, occasionally, a command might modify the document in some
other fashion. For example, moving an object might insert a
`translate` command into the document.

Undo / redo should be thought of as operating on the document history.

# Advantages

## Really simple to implement

All the operations map trivially to methods on the cairo context.

## Simplifies Undo / Redo

Undo is trivial, just pop the last instruction off the end of the
current document.

Redo replaces the last instruction pushed to the undo stack.

## Extensible

We can easily extend this model with new commands, variables,
arithmetic, and flow-control.

## Allows keyboard workflow

Just bind keys to sequences of commands.

## Unambiguous

If I want to move an object, I can do this in two ways:
- Edit the original coordinates in the document.
- Use the `translate` operation.

This approach makes the user explicitly chose which they want.

## _Almost_ modeless

The set of valid operations at any given time can be determined by
looking at the stack.

We can give good feedback to the user about what the valid
operations are by disabling invalid operations in the UI.

If a command takes a point, and the top of the stack doesn't
contain point, then that command is disabled.

We don't need to invent special editing "modes", like a "path editing
mode" mode.

## More powerful than the usual paradigm

Cairo allows a path to contain segments of any type, but most drawing
programs constrain path objects to contain segments of the same type.

With this approach, we don't need to distinguish between different
kinds of path objects.

Plus, we can do all kind of complex / fun transformations by modifying
the textual representation of the document directly.

## Scriptability

Since the document is a command stream, we can capture and replay
these commands to reproduce the application state. This would be a
boon to testing and debugging.

# Disadvantages

## Feedback

In genreal, we can't offer _previews_ of operations, because we don't
know which valid operation the user will choose.

For example, we can't show what a spline path segment will look like
before the user invokes the "splineto" command.

Not clear how big of a deal this is. As long as they can undo, or
adjust the spline control points, it may not matter. If editing is
completely modeless, it might be faster to work this way.

As long as we have undo / redo it might not matter. And in some
limited cases, we might be able to infer the preview.

## Non-linear editing

We probably want to support operations on objects via click-and-drag.

Updating the original point "in place" breaks the undo / redo model
described above, so we either have to support undo / redo via some
other means.

We could introduce the notion of a "cursor" which usually points to
the end of the document. But various update operations would
implicitly move the cursor to an earlier location in the document.

Undo / redo could also just save / restore the entire document state
after update operations like this.

# Variations

## Add a document tree

Some more possibilities might open up if you think of the commands as
operating on the document tree rather than painting to the canvas
directly.

Let's revisit the exmaple:

```
  Operation              Stack

  (0, 0) moveto          [Begin(0, 0)]
  (0, 5) lineto          [Line(0, 5), Begin(0, 0)]
  (5, 5) lineto          [Line(5, 5), Line(0, 5), Begin(0, 0)]
  0.5 setlinewidth       [Line(5, 5), Line(0, 5), Begin(0, 0)]
  'black                 ['black , Line(5, 5), Line(0, 5), Begin(0, 0)]
  stroke                 []
```

The stroke operation consumes *all* the path segments on the stack,
creating a `Path` object, wrapped in a `Stroke` object:

  `Stroke('black, Path(...))]`

This would be appended to the current document "subtree", which might
be the root, or might be a group nested multiple levels down.

The document tree can always be linearized back to a stack-based
representation for display, but we present the tree representation to
the user for the purposes of editing:

```
stroke {
  pattern: 'black,
  path: Path([Begin(0, 0), Line(0, 5), Line(5, 5)])
}

```

## Prefix operations

If we're willing to sacrifice modelessness, we could swap post-order
for pre-order.

The first example might look something like:

  `ellipse (0, 0) 50; setsource #ccffcc; fill;`

This would correspond to the following sequence of events:

  ```
  click "Ellipse"
  click origin on canvas
  click another location to set radius
  choose color
  enter
  ```

Using this model, we *can* provide interactive preview, since we had
to specify the operation up front.

The downside of this is that it we now have verb-noun pattern for some
operations, which is less natural.

I'm not sure whether it's worse to be inconsistent about the
"noun-verb ordering", or to not be able to provide feedback.

This also maps less well onto the cairo api, and loses some
flexibility.

We reindroduce the artificial distinction between different path
objects that you usually find in vector drawing packages, which might
seem more familiar at first. But I think I would find it frustrating
after adapting to postfix ordering.

## Expressions

We can automatically translate infix expressions typed by the user
into RPN. The user can choose to disable this if they actually prefer
RPN.

The expression `0.5 * min(bounds.width, bounds.height)` might be translated as:

  `0.5 bounds 'width . bounds 'height . min *`

Or simply as `0.5 bounds.width bounds.height min *`, assuming we
perform property lookups before pushing to the stack.

In the GUI, the user can type the expresion out, hit enter, and the
result will be appended to the document, and evaluated.

## Repeated Commands

When the user repeatedly invokes certain actions via the mouse, we
interpret this as replacing the previous command.

  `0.5 setlinewidth 0.1 setlinewidth 0.3 setlinewidth`

Is equivalent to `0.3 setlinewidth`. This is is because this command
simply mutates context state, and if no drawing is performed between
calls, then the othe calls to `setlinewidth` are unobservable.

## Snapping and Alignment

Can we do this?

One answer is that you should be constructing your drawings such that
alignment happens naturally, i.e. via parameters and
expressions.

This doesn't have to be onerous, I can factor alignment into helper
functions:

```
: centerof (rect)  // Stack
                     // [rect]
dup                  // [rect, rect]
topleft              // [p1, rect]
swap                 // [rect, p1]
dup                  // [rect, rect, p1]
bottomright          // [p2, rect, p1]
swap                 // [rect, p2, p1]
topleft              // [p1, p2, p1]
swap                 // [p2, p1, p1]
-                    // [(p2 - p1), p1]
0.5                  // [0.5, (p2 - p1), p1]
*                    // [0.5 * (p2 - p1), p1]
+                    // [p1 + 0.5 * (p2 - p1)]
```

This function returns the point centered in the given rectangle:

`0 0 100 50 rect centerof`

Would yield `(50, 25)`

Note: `rect` here isn't defining a path, but creating a rectangle
object on the stack. Let's draw rectangle with an inscribed circle:

```
0 0 100 50       [50 100 0 0]
rect             [rect]
dup              [rect, rect]
dup              [rect, rect, rect]
dup              [rect, rect, rect, rect]
width            [100, rect, rect, rect]
swap             [rect, 100, rect, rect]
height           [50, 100, rect, rect]
min              [50, rect, rect]
swap             [rect, 50, rect]
rectangle        [50, rect]
stroke           [50, rect]
swap             [rect, 50]
centerof         [(50, 25), 50]
circle           []
fill             []
```

Okay, all the stack wrangling is distracting. This is equivalent to
something like:

```
let r = rect(0, 0, 50, 100);
let radius = min(r.width, r.height);
stroke(rectangle(r));
fill(circle(centerof(r), radius));
```

Can we handle the stack wrangling in a sane way via the gui?

It would be useful to provide a couple of features:
- selecting an object should a reference to it on the stack somehow
- an operation like `bounds` would consume the reference and return a rect
- an operation like `center` consumes the rect and returns a point.
- duplicate and swap can also be accessible from the gui


## Iteration

We need to intoduce a new type: Lists.

Inroduce operations:

- `get`
- `put`
- `[`
- `]`
- `repeat`

Operations between `[` and `]` are not executed, but considered
_quoted_.

The `repeat` instruction consumes a collection (tuple, record, or
list) and quoted instruction block. The block is executed for each
element in the collection, with next element placed at the top of the
stack.

`(0 1 2 3)` denotes a list. `0 4 range` creates the equivalent list.

Can also use `cons`, `head`, `tail` with lists.

List elements can be numbers, strings, or lists.

Most often we would iterate over ranges or list parameters.

 `[ticks (Number) "Draw ticks at these angles" (0 1 2 3)]`

Declares a parameter that is a list of numbers, with the given help
text, the final argument is an "example value" which will be used
during development and to render preview thumbnails for the image.

```
 @ticks [
    save
    value_to_angle
    rotate
    (0, @tick_start)
    moveto
    (0, @tick_end)
    lineto
    restore
  ] repeat
  stroke
```

When we invoke the equivalent of `[` in the GUI, the editor switches
modes to indicate we are editing a quoted element.

# Thoughts 5/21/2019

So.... I still like the idea of a using a postfix notation as the
input language for a graphical editor. But I am not sure about
operating directly on the vm bytecode. I want to support a workflow
that's like:

- make a shape you like
- convert the shape to a proceedure
- add a list parameter
- map the shape over the list parameter

Drawing shapes should be something like:

- click somewhere on the canvas
- see feedback region (showing whether we're drawing centered or
  cornered)
- click again to commit the bounding box
- invoke a shape command (rect, ellipse, arc, lineto)
- select a color from the pallet (pushes onto stack)
- invoke stroke (applied to all objects on stack)

## Handling user interaction

It's a subtly different model from the way cairo and postscript
actually work. Subsequent operations often influence the *preceeding*
drawings.

I am building an editor for immediate mode graphics! I am creating the
*illusion* of retained mode by repainting the entire image after each
change to the document. I'm kinda committed to that approach anyway,
because of double-buffering.

I want to avoid the retained-mode model. The state implicit in the
cairo context is more compact, and we don't need to adopt any crazy
"diffing" strategy to minimize changes to the context state. It is
also more "composable". I don't have to invoke a fill or stroke
operation, and it's a useful optimization to "batch" fills / strokes.
Moreover, if we want a "private" context state, we can always use
save/restore.

The difficulty is in mapping a pixel on the canvas back to the set of
operations that have touched that pixel. Only fill, stroke, and text
commands actually alter the canvas, but the real information is in the
path commands that preceed the call to fill or stroke. If I want to
change the color of an object, for example, I have to find the most
stroke recent or fill operation that produced it and insert code to
change the color. Once I do, it may affect the color of other items on
the page, if they were all stroked together. Also, if there was a
previous set_source command still present, but effectively dead.

Still, it may be possible to do a decent with some simple bookkeeping,
perhaps followed by a pass to eliminate "dead" instructions, like
redundtant calls to set_source. The basic idea is that we instrument
the VM in the editor to save a copy of the current path prior to a
fill or stroke, along with the program counter. Now we can hit-test
the path and determine which instruction produced the shape. We would
then move the "cursor" to that point in the program, and subsequent
commands would now operate before the stroke instruction. This both
modifications to the path and changes to the canvas state.

One feature that is often lacking in GUI editors is "batch operations"
on objects. Gui editors are usually smart enough to display a
properties dialog when the selection contains a single object. But
when the selection contains multiple objects, even objects of the same
type, they throw up their hands and display a "general" properties
pane that doesn't let you change the property you want. You're forced
to repeat the same operation on each object, even when the property
you want to modify is common to all objects in the selection. That is
something I want to support, but I'm not sure it naturally falls out
of my byte-code oriented approach. An operation on a disjoint set of
objects would essentiall be a multi-line insert operation. I can think
of ways to do that, but it definitely sounds complicated, and would
have to be limited to changes to the context state, rather than
changes to the path.

Another thing that's not entirely clear is how you would handle
something as mundane as click-and-drag to move an object. A naive way
would be to simply insert a "transform" prior to the start of the
subpath (with matching save / restore), and sometimes this will even
be what you want. But in other cases what you really want is to change
the path itself. I'm not yet sure how you would automatically know
which the user wants, and what the best way to distinguish between the
two is. There may or may not even be a visual difference. But there
will be a structural difference, and that's the kind of thing I care
about. You get different behaviors under scaling and mutating
parameters with the one vs the other. I can handle things like "scale
independent strokes" implicitly by being careful about which I choose,
and for right now I believe that this is the best way to handle a
feature like that.

The clue that this style of editor may truly be a Bad Idea is that
inserting code at aribtrary locations will alter the stack layout in
ways that are hard to predict. One way to at least prevent breaking
the code completely is to insert operations with preceeding "dummy"
arguments, that later get "backfilled". On the other hand, if the user
begins by inserting an operand, we append a dummy "drop" operation
which gets "promoted" to the next opcode the user enters (at which
point the arguments may need to be reconciled). I doubt I will get
this right on the first try. I may have to introduce a "top value"
which all instructions will accept without crashing the vm. As long as
they do something sane when they encounter it, I only need to be
concered with matching the airty of arguments. Also having a
distinctive marker in the code would make it easier to scan for. Stray
"default" values remaining in the file would highlight bungled edit
operations.

## On consistency

Graphic coordinate systems for image formats should *always* be with
respect to the center. Not the bottom left, and certainly not the
top-left (which inverts the usual meaning of the y axis leading to
no. end of problems). If you're designing a new image file format, or
a new graphics library, keep this in mind. Even if it's not the
default mechanism, conceive of, and think of your work as being with
respect to the center of its own coordinates. This is something I will
be opinionated about for the rest of my life. I never really thought
it mattered before, but I had an epiphany last night: I could dispense
with a lot of math, and pointless writing and calling of routines like
`center`.

I understand why computer cordinates are y-flipped, with the origin in
the top-left corner... it has to do with how analog TV worked, and
early computers with unsigned counters, etc, etc. It's an excellent
implementation strategy, but it poisoned the brains of computer
programmers after the 1960s. The vector display used on the PDP1 had
the origin at the center, which was just happened convenient for that
particular hardware (it was a radar screen). But it was also
mathematically superior. This whole project is about being able to
arbitrarily combine images that are themselves dynamic, and this is
the right way to think about doing that.

Advantages of centered coordinate system:
 - image will be concentric to screen (or parent container) by default.
 - The default effect of rotation rotates the image about its own
   center, not some arbitrary corner (of the drawing as a whole).
   - 99.99% of the time this is what you mean by rotation.
   - even when you don't, it's probably some other point -- almost
     never one particular corner.
   - vector drawing programs got this completely wrong for decades.
 - even if you do want to create an offset or asymetric image, it's
   still easier to think in the context of being "shifted" from its
   natural center (think how the mouse cursor points "toward" the
   actual cursor coordinates, rather than being centered on them).
 - layout of objects gets easier, since 99% of the time you want to
   align the *centers* of objects, not the *edges*.
   - an object's position always refers to it's "visual center".
   - if you can tell the difference between center and edge alignment,
     it's almost always centre alignment you want.
   - And when you don't, it's usually better to draw the object itself
     as offcenter, since it's probably what feels "natural" for the object.
   - we don't need the object's dimensions just to center it. Only to
     space it.
     - most of the transforms in an image are about keeping things in
       the relative coordinate system of parent elements.
  - aids with aligning proceedural and raster patterns (gradients,
    clipart). The pattern coordinates are the image coordinates by
    default, proceedural patterns are often symmetrical around the
    origin.

The previous epiphany about coordinate systems made me wonder if I
shouldn't strive for resolution inedpendence. I have previously
thought of this as something mainly for print publishers, but I see
now resolution independence the only way to reach the broadest
possible audience. It is also the best way to future-proof my work
against changing display standards, hardware, etc, making sure all
this eventually pays off somehow.

I normally am a geek about getting perfect 1-pixel hairlines in the
device pixel space. This is becauese certain effects, like the "3D"
beveling of edges work best when the hilight / shadows are exactly one
pixel. Other times the best way to cover up certain visual artifacts
is with a 1-pixel hairline. But these cases are rare, and I can just
have way to explicitly set a hairline, rather than implicitly relying
on it as a side-effect of the particular coordinates system. And
actually, when it comes to things like "scale independent strokes",
they actually have *more* meaning when your're working in physical
units. The whole point is to have a line of "reliable" thickness
regardless of the display size, and there will be too many unpleasant
surprises if the coordinates are arbitrarily based on screen
resolution. Just look at what happened to windows on Retina. Even
pixel art, which has become an artistic style in its own right, isn't
really about pixels any more. It's more about "drawing with a limited
number of little squares and and even more limited palette." So I am
adopting the point mm as the official unit size. Either that or the
point. I'm kinda loathe to use such an archaic unit, but it has
advantages for working with text in different fonts. I think that most
of the layout decisions are actually driven by the placement of text
elements. It's also what poscript natively uses. But working in mm
would benefit people with... you know, rulers and tape measures. Of
course you're always just an affine transformation away from whatever
crazy coordinate system you want to use. The important thing is just
to define *something* as the nominal units so users can know how to
convert to their preferred units. I guess there might be some
arbitrary choices that might lead to fewer rounding errors or some
such nonsense. From that perspective, multiplying by 25.4 seems better
than dividing by 25.4, which is really multiplying by whatever 1/25.4
comes out to in IEEE floats whenever matrices are involved. I could
just piss absolutely everyone off and say the default units are
inches :Px

I think I may have to stop short of color managment if only because I
think I've piled on enough implementation headaches: I gotta make sure
to set up the right transform matrix by default, and figure out how to
get DPI information out of libdrm (and provide a way to manually
override and calibrate, because monitors are liars). I also have to go
and re-examine *all the graphics code I intend to keep*. Also users
don't, as a rule, pay so much attention to color correctness, and it's
a much easier thing to fix after the fact. For the time being I will
just work with the "websafe" palettes that people have come up with
that tend to look okay on most screens.

## Splines

I've been reading about Bezier curves and splines. Not because I want
to re-implement them, but because I want to understand them
better. Like one thing I didn't realize is that translating a bezier's
control points performs the same transformation on the curve. I guess
that might seem obvious, but I didn't *know* that for a fact. What
other useful facts about them am I unaware of? Also, I am interested
in hardware acceleration, and the least invasive approach of doing
that would be to simply figure out how to render vector graphics
directly on the GPU. Contrary to what some people have claimed, there
*are* techniques for doing this, they just aren't widely known. Not
even bleeding-edge, but I'd be doing my own impelementation most
likely, either contributed upstream to cairo, or else as a stand-alone
project.

There might also be patent concerns in some cases, and hardware
limitations that make the whole thing a wash in terms of
performance. But it seems like an interesting topic, and would be a
real contribution to an open source project I have gotten a lot of
miles out of. And it could be a game-changer for cairo if it had fast
accelerated rendering out of the box.

Anyway, in order to do that I have to bite the bullet and do some
math. At least think about these objects in a deep way. Maybe
implement a toy rasterizer to get a feel for it. Then think about how
the gpu could be used to speed up the rendering. I wonder if newer
APIs that expose the hardware more directly (Vulkan, OpenCL) are
actually better for this purpose, or if the people who claim it
doesn't scale to the GPU have just not looked at it closely
enough. For example, subdividing sounds like it could be done on a
geometry shader, and I've seen some tesselation-based approaches that,
while not exactly efficient in terms of number of polygons, at least
seem to run well in practice. The thing to keep in mind is that GPUs
can render just mind-boggling numbers of triangles, and vector
graphics tend to be rather sparse. And, finally, that a lot of vector
graphics are polygonal anyway, and optimizing those leaves more time
for the smooth curves in your file.


## The rabbithole

Let me acknowledge just how deep I've gotten. I started by wanting to
do an onboard dash, which led me to buildroot. Meanwhile the the
RaceCapture app was too slow for my hardware, so I decieded to write
my on application. But to do that I decided I needed to invent a file
format, so I could configure the dash. And now I'm looking at writing
tooling for a file format I have yet to settle on. And there's still
work to do on the data processing side, on the actual hardware
installation, etc, etc. The code I already have would work for the
time being,

But what I realized is that I've wanted to write my own vector drawing
program for a long time. That's because most of them have frustrating
limitations. They focus either on exact mechanical drawing, or totally
free-hand artistic drawing. I have yet to see an application that
really does a good job of mixing the two.  I *want* that drawing
program.

I have also have not seen one that really makes repetetive or
parametric elements intuitive. The frustrating thing about drawing by
hand is that drawing directly is that drawings are often structured or
repetitive, but the facilities most packages provide for replicating
elements are limite to a small number of basic patterns, with no
support for defining custom patterns, and usually pretty clunky UI
supporting the feature. If they offer anything at all. Oftentimes they
resemble more of a "macro" function that is difficult to edit or
change afterward. It's often easier just to manually position the
objects, in some cases calculating and entering coordinates by
hand. When you have all this computing power! In may cases it's
*easier* to edit the underlying svg file than to rely on the gui. I
think once or twice I may have even edited a PostScript file in a text
editor.

So maybe this isn't about the Palatov anymore. Maybe this is about
finding something sufficiently challenging to work on. Or maybe it's
about scrating that personal itch. Or maybe I'm just scatterbrained.

# 5/23/2019 Update

I've spent way too long trying to write a proper lexer that can be
called incrementally. It's really pretty straightforward to write an
iterative lexer that scans tokens in a loop, or recursively.

It's really hard to implement a lexer that is called sequentially, at
least in python. You have to encode the state explicitly, and it's
actually really hard to do that. I want that because I want to support
using the same lexer in "batch mode", via the command line and / or
unit tests as well as interactively from windowing system
events. Using coroutines would be pretty natural, and pretty
efficient, and super flexible: you can call the lexer either with a
list, or with a "stream" of events that is asynchronously written to.

Python 2's generators can *almost* get you there, but
it would be nicer with the *yield from* syntax added in python 3, and
nicer still with the *async generators* new in 3.7.

So now I at least have to consider stopping what I'm doing and
starting over in at least python 3.5. Or do I? I know I should get
used to python 3 eventually, and with asyncio, they may finally have
the secret sauce that brings me into the fold. But.... ugh... i've
gone down too many rabbit holes already.

But it's worth remembering for the future. I have had the idea for a
while now that UI programming really needed a fundamentally different
model than the callback-driven one. I suspect that a future UI
framework will come along in which everything is done via coroutines.
