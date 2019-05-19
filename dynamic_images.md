# On Dynamic Vector Graphics

This is file collects a bunch of ideas related to "dynamic vector
graphics".

If we want to support user-defined gauges, we need an image format
that supports "dynamic images".

I define a "Dynamic vector image" as some kind of stand-alone image
representation which exposes one or more residual "degrees of freedom"
to the renderer.

In my conception, this does not employ any embedded or external
scripting of otherwise static images. Rather, the dynamic elements of
the image are expressed in directly in terms of free parameters. These
parameters are *evaluated* in some *environment*, and the resulting
values used to construct the image.

In this model, rendering is idempotent. Rasterizing a given image
image within a given environment always produces the same image. This
formulation omits user interaction from the image representation. The
parameter values are immutable with respect to the renderer, and it is
up to the application to manage them.

Example uses:

- Interactive representations of mechanical systems.
- Animation (special case where only paramter is *time*).
- Instrumentation (motivating use case).
- Simplifying construction of custom UI elements.

# Motivation

Users will want to to easily tweak the dashboard to their liking. This
is mainly about layout and channel configuration, but not entirely.

Some users will want to design custom gauges.
  - branding
  - novel visualizations
  - implement missing features
  - some users will dislike aesthetics of stock gauges.

I want to allow the community to contribute custom gauges.

I want to to allow for unanticipated uses.

I want to use the same representation for all gauges, regardless of
who contributed them.

Want to avoid re-inventing the wheel if possible.

Users may want to share entire top-level dashboard configurations. It
should be easy to edit these as well.

# The Problem Statement

These are the problems we are trying to solve:
- provide an efficient representation for display of dynamic images.
- provide configuration freedom for users.
- provide authoring tools for designers of guages.

# The "Gallery" model (for end users).

Users pick and choose gauges from a *gallery* of gauges. Each gauge
has a representative thumbnail.

Inspecting a gauge in detail, user is presented with an interactive
"configuration wizard" exposing the full featureset.

User tweaks options until satisfied, then commits the gauge to their
layout. User can return to edting the configuration for any gauge at
their convenience.

# The "Constraint" model

Most CAD programs use geometric constraints to allow for dynamic
geometry. Unlike cad, we would want to deliberately under-constrain
our geometry, and expose the remaining degrees of freedom as
parameters.

I have not seen an artistic vector drawing package that used
constraints effecively. Only engineering CAD. I'm not totally sure why
people think artists don't need or want constraints. They are super
useful.

It's just like regular drawing, except you have a few more tool in the
toolbox for defining constraints. It generally works well, at least
until you do hit a bug or do something screwy that results in the
constraint solver barfing all over your drawing. Then you have to
often manually delete the conflicting constraints, and the UI for this
is usually barely functional.

# The "Draw with code" model, aka. "Images are procedures".

I.e. you write code, and the code defines theimage.

Your work flow is a compile -> test -> fix cycle. You need an editor,
preview application, and a way to change the free parameters and
regenerate the image.

This can be a simple editor-driven workflow, where you manually
re-generate your target images, or there could be some fancy IDE that
integrates all the pieces more-or-less seamlessly.

The benefit here is that it's completely unambiguous. The downside is
that it's totally awkward to draw images this way. In particular, it's
hard to visualize non-trivial coordinates. To manage this, you need
abstraction mechanisms, like functions and named constants. You'll
want strong typing, and you'll want container types. You'll end up
reinventing python. On the other hand, you can start by parameterizing
everything, and then hopefully our image is more likely to correclty
handle all the degrees of freedom, since you had to account for them
them from the beginning. As you get closer to finished, you can
can just not expose quantities that don't change.

# Hybrid approach: extend WSYWIG vector drawing with control flow

This is uncharted territory.

For example, to make a dial with a dynamic needle:

- I draw some arbitrary circle
- I draw a line, one point centered on the circle, the other on the
  circle's perimeter.
- I select the circle, and bind its radius to free parameter $R.
- I select the line, and bind its length to the expression $R * 0.5.
- I rotate the line, but then bind the angle to $A.
- I edit free parameters $R and $A, which were autocreated, providing
  demo values and doc strings.
- I save the file.

This falls down for things like guage scales, which vary according to
parameters given by the user. Not every dial has the same number,
type, and spacing of tick marks. The user might want to allow for
embellishments, etc.

- I create thick line to represent a tick mark.
- I select the tick mark
- I create a radial pattern element, centered on the circle, which
  consumes the tickmark.
  - But the redial pattern needs a *collection*. What are the elements
    of the collection? That depends on how the pattern is used.
    - loop constructs in procedural programming avoid the ambiguity.


Orthogonality:
- Every value can be bound to an expression involving free parameters.
- Some values can come from the mouse.
- Some values come from the selection.
- Some need to be typed in directly.

Modularity:
- Small number of primitives
- Each primtive is supported by hand-crafted UI elements.
  - graphical "handles" in the drawing layer, "properties" editors in
    the sidebar.

Can we design a graphicaly driven expression editor that doesn't suck?
I have never seen one, but that doesn't mean it's impossible.

Can we substitute geometry for math. Here's an example that avoids the
need for an expression:

- I create an arbitary circle
- Bind the radius to free parameter $R as before
- I "derive" a new circle from the original.
- I resize the new circle. Becase this is a derived shape, all its
  properties are interpreted relative to the base circle.
  - I could bind the new circle's radius to another free parameter,
    and it will interpret the parameter value as an offset.
- I change free paramter R. Both circles change size. The size
  difference is maintained.
- I could scale or translate the circle instead, having a similar effect.
- I can easily combine transforms using multiple free paramters.

Can this approach naturally extend to control-flow constructs like
conditional elements and patterns? Can we:

- Distribute a collection objects along a path
  - Optionally normal to the path, or aligned to a static vector.
  - Using explicit positions, or even spacing
  - Using a finite set of arbitrary elements, or infinite generative
    sets.
- Create a collection of objects for some array of scalars
- Conditionally draw one shape or another based on a parameter value?

Seems doubtful.
"Drawing with code" can do all of this stuff, with fewer core concepts.

How do I combine mutliple shapes

# Considerations

Do we want to focus on free-form vector graphics? Or do we
intentionally want to limit the drawing model to something that's
efficient in the context of GPU acceleration?

Note: could be a web-based project, or native. Or a plugin for
existing software. Or a stand-alone tool that converts from an
existing format.

Mouse-driven interaction is slow for everything except
drawing.

Interaction that frequently switches between keyboard and mouse is
pessimal.

A one-hand-on-keyboard + one-hand-on-mouse is likely optimal unless
you frequently need to enter exact numbers.

The more complex the format, the longer it will take to implement. And
the more tooling will be needed to debug complex configurations.

Custom tooling will limit accessibility of design. But adapting an
existing format like svg will require supporting more features than we
really need.

I need to be able to export names and symbols defined in gauge files
into the top-level config. I need to be careful here, because having
to traverse a dependency graph will may impact load times.

I need to consider how users will update their config:
 - Support direct editing via touch-screen or mouse?
 - Support some kind of remote configuration?

Does top-level config even need to be file-based?
 - could it be event based?
   - would better support common case of automatically extracting
     config from rcp
 - could it be passed in as arguments from a shell script?
   - could make expressing conditions awkward.


What kind of runtime behavior do we even want to allow?
 - at least alerts, and flipping to each page defined by the user
 - runtime config means config has to be mutable. immutable config was
   a simplifying assumption.

I don't want to impose too many conventions on how gauges are
produced. Gauges should be able to publish their configuration
interface, and udashboard should be flexible and not require all
gauges take the same parameters. Dials need different options than bar
gauges, or idiot lights. Something like a traction circle would need
to accept multiple channel inputs. I do want to allow for novel
deisgns.

Graphical tools are way down the road, so in the interim there needs
to be a sane text-based format, or I need a simple workflow for
adapting files in an existing format.

Things like JSON, Yaml, Ron, etc, are problematic as config languages,
because typical implementations make good error reporting impossible
(loss of source information).

Gauges are typically a "sandwich" of layers: a stack background layer,
a dynamic indicator layer, and possibly a static "foreground"
layer. This structure could be exploited by a compositing back-end to
speed up rendering, as only the dynamic layers need to be
re-drawn. The background layers can be re-used between frames. Not
sure if I really need to support this, but I want to leave open the
possibility in the short term.

Above all else, I need to get something working for myself Yesterday,
so I can't go down too many rabbit holes.

# Approaches

## Embed a scripting language into an existing format

This is the obvious choice, but don't know what scripting language I
would choose.

One problem here is that we actually don't need / want turing
completeness.

Another problem is of naming / referring to graphical elements if the
underlying format doesn't support that.

## The template approach

This might look something like "handlebars"-style templating embedded
into an SVG Document.

The idea here is that at load time we expand all templates into a flat
list of "primitives", some of which are dynamic. I'm being
deliberately cagey about what constitutes a "primitive" in this
context. They could just map onto the cairo API, or they could be
higher-level "elements" provided by the implementation (for example,
to simplify syling).

So for example, a Dial gauge could decompose into:
- ellipse backround
- text label
- a tick mark for each "tick" in an array passed to the template
- dynamic needle "primitive" provided by implementation

## Extend cairo_script with flow control constructs

Cairo already defines a "script" language, though it seems to be more
like PDF than PS. Hell, it's probably intended mainly to aid the
implementation of PDF rendering, but also for testing and debugging.

One idea would be to extend cairoscript with the postscript
control-flow constructs, and turn the cairoscript interpeter into a
true stack machine. This could then be contributed upstream to the
cairo community, probably behind a feature flag, or alternative API
since the vast majority of the time this would be undesirable.

The other way is to define an intermediate representation, which
implements the control flow constructs and arithmetic, but which then
flattens to cairoscript.  This could simply be an extension of the
existing bytecode.

Not sure the latter apporach is actually faster / easier / better than
simply writing a custom interpreter whose operations map directly to
the cairo api. But maybe leveraging cairoscript internally will pay
off in other ways. On the other hand, I'm not sure that cairoscript is
intended to be a stable interface. More a debugging tool.

Which brings me to the final problem here: there's no stable text
representation for cairoscript, it's an in-memory representation. I
would still need to handle lexing and parsing for cairoscript
itself if I wanted a stable syntax.

## Define a custom DSL for dynamic images

Implementation wise, it ends up being similar to the above, but we
have the added flexibility / responsibility of defining the featureset
ourselves, and keeping it in sync with the cairo api (or whatever
rendering engine).

The custom DSL could be declarative, like SVG, or imperative, like
PostScript. It can mix aspects of both. But it can only contain
features that I reasonably have time to implement.

## Use HTML5

If I had wnated to do this in the first place, I would be trying to
port ElectronJS or Skia to use libdrm. I wouldn't be writing in Rust
against libdrm.

This could still be a valid approach: take an existing framework and
add drm support.

## Use SVG + CSS

If somone else was willing to do the implementation, I would just
accept the contribution and move on. The main reason I'm avoiding SVG
is that it's a big, complicated spec. Things like `librsvg` do exist,
but I'm not sure that will be flexible enough for dynamic content.

## Hybrid approach

I could define a wrapper language that lets you embed or reference
images in existing formats.

## Punt

I could define some really basic format that's based on a really
simple model, like one where gauges just define static layers. The
static layers would be rendered once, one buffer for each layer.

The dynamic layers would allow a limited number of transformations,
which would be easy enough specify with a very simple config. Needle
type gauges could be implemented with rotation / translation. Bar type
gauges with clipping. Only the background layers would be
full-color. The foreground layers would be mask layers for a
dynamically chosen pattern. These layers only need to provide a path
or an alpha channel.

This would only rquire a very simple configuration format, where we specify:
 - basic metadata
 - the image source for background layer
 - the number of input channels
 - for each non-background layer
   - whether a layer is static or dynamic
   - the path and / or alpha mask of the layer, which is always static.
   - if dynamic,
     - the transform operation to peform on the layer:
       - scale, rotate, sheer, skew, clip
         - clip paths would be dynamic, but could be constrained to a
           few simple cases.

This is a more top-down authoring scheme, where you don't draw the
gauge directly, but merely combine existing assets.

This is necessarily more limited than arbitrary dynamic vector
graphics, but would probably work in the vast majority of cases.

Definitely falls down for multi-channel instruments, like a traction
circle.

# Ideas

## Basic-like proceedural language

The bytecode interpreter would be stack-based, but I don't really want
to make people program in Forth while I figure out how to write an
editor. I want something friendlier, but that can be easily translated
to and from the stack machine's bytecode. Also, it needs to document
its own interface, so that we can validate the configuration at
runtime.


This is my Pie-in-the-skye rainbows-and-ponies Dream:

```
"""A traditional round gauge."""

image dial

type Style = {StyleKey: Pattern}

type StyleKey
   = outline("Outline")     """Stroke pattern for the gauge outline."""
   | indicator("Indicator") """Fill pattern the gauge needle."""
   | tick("Tick")           """Stroke pattern for the gauge tickmarks."""

param $bounds: Rectangle
"""Where to render this gauge."""

param $channel: Channel
"""Determines the position of the dial."""

param $range: Interval
"""Defines the minimum and maximum value we can display."""

param $arc: Interval(0, 2 * $PI) = Interval(225 * $DEGREES, 315 * $DEGREES)
"""The start and stop angles of the gauge"""

param $style: Style
"""Controls appearance of gauge."""

param $ticks:  [{label: String, angle: Number: $value.domain})]
"""The sequence of tick marks to draw around the dial."""

param $offset: Point: Within: $bounds
"""The offset from the center of the bounding box. Give (0,0) to center."""

let $radius = max($bounds.width, $bounds.height)
let $needle_rad = $radius * 0.8

// really should adjust based on font size.
let $tick_rad   = $radius * 0.5

func angle($x) = 1.5 * $PI * (1 - $range.percentOf($x))

layer "background" {
  translate -($bounds.center + $offset)
  arc (0, 0) 150 0 $PI
  stroke $style.background
  foreach $tick in $ticks {
     save {
       rotate angle($tick.value)
       moveto $radius
       linecap round
     }
  }
  stroke $style.tick
}

layer "indicator" {
  translate -($bounds.center + $offset)
  rotate angle($value)
  moveto (0, 0)
  lineto ($origin.x, $radius * 0.8)
  stroke $style.indicator
}

```

### Notes:

Self-documenting:
- We can use the metadata to generate dynamic UI elements.
- Python-style docstrings.
- Rich type information lets us choose appropriate configuration
  widgets at runtime.
- layer hints are given so that we can optimize rendering. We know
  which free parameters are used in each layer. We do not need to
  re-draw instrument layers for which no free parameters have
  changed. We can render layers with the same name onto the same
  intermediate surface.

The self-document

### Language Design

- no mutable locals
  - graphics state is implicit and mutable

- Type annotations based on flowjs syntax
  - primitive types: string, number, enums.
  - generic types: tuples, maps, ordered collections.
  - map keys can be enum or string
  - type annotations describe value sets


### On Types as Sets

Types can be thought of as sets.

One way to represent a set is as a function on a given value which
returns true if the value is a member of the type. We can then define
operations like |, ^, &, -, as trivial combinators. We can also define
composite types like sequences, tuples, and hashes as composite types.

This representation gives a formal definition for types that's easy to
work with, and also gives a straight-forward way to validate input at
runtime:

- The colon (:) is taken to be the "subtype of" operator. Any value is
  a subtype of its own type, and singletons are always subtypes of
  themselves.

- The lift operator (::=) lifts a lambda expression returning a bool
into a type.

The notation `t ::= x => y` means: "there is some type `t`, for which
the membership of some value `x` is given by the expression `y`.

I.e. `y` is a boolean expression over `x` that returns `true` if `x`
is a member of the type `t`. In this case I'm using javascript's
expression syntax, because it distinguishes between `is` and `==`
concisely.


#### Enums

Enums are interned strings. Enums are singleton types. `foo x ::= x is
  foo`

#### Booleans are Enums

`Bool ::= x => x is true || x is false`, (equivalent to `Bool:
true | false`).

#### Top and bottom types.

`Value` is top type, is the union of all known types. `Void` is the
bottom type.

Trivially, `Value ::= x => true`, `Void ::= x => false`.

None is just an enum, i.e. a singleton value.


#### Strings and subsets of strings.

String: set of all strings. Not sure how to specify its definition,
since I think it's fundamentally implementation-defined.

It'll end up something like `String ::= x => typeof(x) is "String"`,

Regex would also be an implementation-defined builtin, `Regex(pattern)
::= x => pattern.match(x)`

Are there other ways to specify subsets of strings?

#### Numbers

Number supertype:

Real: set of all 64-bit floats

- `Range(l: Real, u: Real) :: x=> x >= l && x <= u`
- `Integer x ::= x == floor(x)`
- `Quantize(w: Real) ::= x => x % w == 0

Assuming `%` is floating-point modulus, aka gamma function.

#### Type operators

Subsetting generalizes to tuple-types and map types:
  {T}        x ::= x => T(x[0])
  {T, TS...} x ::= x => T(x[0]) & TS(x[1:])
  {K: T}     x ::= x => all(x.keys, k => K(k)) & all(x.values, v => V(v))
  [T]        x ::= x => all(x, e => T(e))

- Subsetting operators
  ~T         x ::= x => !T(x)
  T | U      x ::= x => T(x) || U(x)
  T & U      x ::= x => T(x) && U(x)
  T ^ U      x ::= x => T(x) ^ U(x)
  T - U      x :        T(X) & (!U(x))

### Questions

So, given all that, can we use these definitions to automatically
genreate good UI? Or do these definitions need to be translated into
an equivalent generative form to answer questions like:

For numbers:
- is the type infinite? (--> render an input box)
- is the type on a contiguous range? (--> render a slider, with appropriate scale)
- is the type quantized (--> set appropriate step value on the slider)
- is the type disjoint (--> gracefully handle illegal values)
- is the value set very small (--> fall back to combo box / popup)
- is the value set completely empty --> show error message

For enums:
- is the size of the type large? --> use listbox with completion
- is the size of the type small? --> use popup

For flags:
- use combo box / popup / listbox with multiple selection.

For strings:
- label, or styled text input.
  - need extra type hint for how text is to be used.
- if regex, use regex-validated input box.

For | union types, present a choice widget:
- radio boxes switch between alternatves,
  - nest the underlying type within the radiobox or on a subscreen.
- or tab panes

For {...} record types, they would be a grouping of labeled controls.
- The legal field names need user-visble strings and doc comments.
  - The above example accomplished this by using enum keys.
  - Records with string keys would be displayed as tables instead?
  - Keep in mind records can have nested.

For tuple types, it depends:
 - (Real, Real) => could be a point on the screen, or a drag offset, a
   pair of input boxes. The structural type information is not enough
   here.

For list types, need multiple controls: a list box with +/- buttons. How would
the user provide this input?

The trouble with all of this is that it can deeply nest, which
frustrates attempts to automatically lay out components in a sensible
way. This was my experience trying to do automatic config UI in
Pitivi. You get problems with awkward positioning of components,
components rendered below their minimum size, and / or else tedious and
inflexible interation.

Generative UI can only ever hope to be "good enough". It will always
either be specialized to certain narrow uses, or else it's a mediocre
and frustrating input scheme.

A good example is kconfig, which this is starting to resemble. Though
kconfig could be a lot better if it had better type information.

This suggests perhaps simply using kconfig, or something like it.

What I don't want to do is specify the complete UI description in the
image file, since that couples us far too tightly with a particular
style of UI.

# Other random observations

Types have a lot in common with grammars. And I've always thought that
grammars have a lot in common with the hierarchies of UI widgets that
we construct. Which seems to suggest you could mechanically generate
widget hierarchies from type descriptions. The resulting UI is
probably not usable for the average person.

What is usability? There's usability in the general sense. There's
"intuitive to the naive user". There's "optimzed for the veteran
user". The notion of usability is always relative to your intended
audience. The unix philosophy is to not make any assumptions about
your users beyond a baseline knowledge of unix itself. You shoe-horn
your UX into the existing notions of files, commands, and
pipelines. Everything has a textual representation.

Things get more complicated when you realize you have different kinds
of users: there's the maintainers who will package and ship your
stuff, contributors who add functionality using tooling you provide,
and then there's the end user. Sometimes you are all of these things
yourself.

It's easier to get command-line UI right than graphical UI right. It
seems easier to get textual UI right than graphical UI right, but
designing languages, even simple config languages, is about as hard as
designing a good interactive graphical UI. It's all about
placement. In both cases, deep nesting is bad. Users prefer a
vertically-extended interface. Graphical UI brings with it
considerations of visual composition, and mouse / touch
interaction. Computer languages bring consideration of syntax,
scoping, and meaningful error messages. Both styles need to provide
feedback about mistakes to the user.

There's a weird overlap between mouse-driven UI and stack languages:
They're both noun-verb style. In stack language, you push nouns onto
the stack, and verbs pop them off, replacing with a new value. This
can feel like mutation. In direct manipulation, you select nouns, and
issue commands which mutate them. Can we implement direct manipulation
UI on top of a stack-based vm?

Think about it like this:
```
click canvas          --> poke mouse_pos         [p1]
shift + click canvas  --> push mouse_pos         [p2, p1]
click circle command  --> call create_circle     [&Circle(&p1, &p2)>]
click derive command  --> call derive            [&Derived(&Circle(&p1, &p2)), &Circle...]
type 0.5 enter        --> push 0.5               [0.5, &Derived(...)]
click scale command   --> call scale             [&Scale(0.5, &Derive(...), &Circle...]
```

This is a more-or-less concatenative approach. The selection set
becomes an ordered stack. Operations don't have to consume the whole
stack, and the results of operations go back onto the stack (remaining
part of the selection).

Most commands pop arguments and push return values from the stack. A
few commands  mutate the top of the stack. The document is
constructed as a side-effect, as a tree of primitives.

The Click-and-drag idiom is useful in the following situations:
 - specfying a pair of points
 - specifying a movement
 - specifying an offset
 - specifying eliptical, or polyginal regions:
   - in each case, either centered or from one corner
   - in each case, either square or nonsquare aspect

It gets confusing when we try to think about what the document
actually is.

- Is it a "feature tree" built up as a side-effect of executing commands?
- Is it the contents of the stack after executing the commands?
- Is it simply the sequence of commands?
- Is there a the model for editing the command sequence beyond undo/redo?
- Let's say I want to move a circle. How do I distinguish between:
  - Applying a Translate() to the circle
  - Modifying the circle's original coordinates
  - Which of these will result in derived / cloned shapes also being
    translated?

These kinds of considerations are why systems like LaTeX have
survived: text-based representations side-step some of these annoying
questions. Tooling can usually mitigate the pain of this approach. The
user can choose their paradigm: build up content directly via
interactive REPL, or edit the content directly in a text editor.

For mouse-driven input, we have to consider not just the data
representation, but all the ways in which we are allowed to change the
data. Most of the commands are invoked via menus, and these provide
the same sort of semantic distance as typing out commands at a
prompt. Direct manipulation can really only be used as sugar around
the common operations.

 = Do we allow editing that

