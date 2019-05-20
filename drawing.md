# Editor Design

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
