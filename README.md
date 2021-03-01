# &mu;Dashboard

&mu;Dashboard aspires be a feather-weight embedded dashboard application for
motorsports use. It is in the earliest stages of development. It is not yet 
ready for competitive use.

![Screenshot](screenshot.png?raw=true "Screenshot")

## Roadmap

*update: 2021-2-28*

The master branch hasn't been touched in a while, but is functional. It may or may not not build on newer rust. Some libraries required nightly as of may 2019. I would like to switch back to stable rust as soon as dependencies allow.

Most of the recent work is on the `vm_render` branch, but it's a bit of a mess. That branch is going to be removed soon, and the VM-related aspects will be ported over to the uDLang project. The work un-related to the custom virtual machine will eventually land on master, and the result will be tagged as version 0.1-mvp.

The majority of my effort will be directed towards uDLang, at least until it is featureful enough to host uDashboard.

For more details, see `roadmap.md`.

## Goals
 - Rock-solid stability.
 - Be a usable dashboard for club racing / time trials / budget endurance teams.
 - Produce appealing graphics and reasonable framerates on modest single-board systems.
 - Minimal startup time.
   - debug builds launch in < 2s on a modest arm host.
 - Bare minimum of runtime dependencies.
 - More configuration flexibility than you can shake a stick at.
 - Serve as a test-bed for instrumentation design.

## Dependencies

- linux computer of some sort with video output.
- kernel with at least drm dumb-buffer support for your GPU.
- drm-rs + dependencies
- cairo-rs + dependencies
- ron + serde (for configuration)

## Data Input

uDashboard can display data from any source, so long as it's `stdin`
and so long as it's json.. The input is read line-by-line, and each
line is expected to contain a single JSON map. The keys should
correspond to channels in your config. The values should all be
numbers.

- See `scripts/simulate.py` for a script which generates test data.
- See `scripts/replay.py` for a script which will replay data from a text file.

## Configuration

Configuration is currently based on Ron, which is similar in spirit to JSON,
but more structured. The exact format is still in development. It's a balancing
act between flexibility, simplicity, and ease of implementation. Currently, 
I'm shooting for a representation that captures the vast majority of use cases
with the smallest feature set, intentionally avoiding turing completeness.

### Gauges

Gauges come in several flavors. You have multiple knobs to turn with respect to
the size, appearance, and style, for each gauge. You can position them freely on
the screen if you want to, but there's a little bit of help available via the
Grid layout method.

### Pages

A page is simply a list of gauges. You can recycle the same gauge across multiple 
pages.

### Channels

Channels are merely an index into the data stream. You an assign the same 
channels to multiple gauges. Channel data can be scaled by arbitrary polynomials.

### Alarms

Alarms in &mu;dashboard are implemented via the interaction between *Conditoins*
and *Styles*.

The goal here is to reduce distracting false alarms, by allowing the user two freedoms:
- ability to model complex conditions that require multiple inputs
- ability to control *exactly* how a condition is shown.

### Conditions

Conditions are logical assertions about the value of a channel or other condition.
For example: `"ECT_HIGH": Gt("ECT", 205)`, says that the "ECT_HIGH" condition is `true`
if the channel "ECT" has a value greater than `205.0`. Conditions can be combined
via `And` and `Or`, and `Xor`.

The `Within` conditional allows using a second channel to define an envelope
around a given channel, for example: engine temp vs. ambient temp. Oil pressure vs rpm.
Oil pressure vs. engine temp.

Some limited mathematical functions, in the form of filtering may also be added. This would be
useful if, for example, you want to suppress alarms caused by a momentary dip in oil pressure,
while still being sure to receive a timely warning about sustained pressure loss. It may also
be useful for providing cockpit feedback about other complex data, such as lambda values.

### Styles

Styles describe limited aspects of a gauges appearance, namely it's colors. Gauges
have a default style, and then, optionally, a style for each defined condition. This
is how alarm conditions can be implemented. This allows any gauge on the screen to react
to any condition, if the user so desires.

## Questions you probably have

### Why software rendering?

I'm open to the idea of supporting GPU acceleration, but it's very low priority.
 
 - Freedom: I want this project to be free software.
 - Portability: while embedded GPUs are ubiquitous, *support* for those embedded GPUs leaves a lot to be desired.
 - Peformance: believe it or not, GPU acceleration does little for vector graphics, except in certain limited cases.
 - Design freedom: I don't want dashboard designs to be limited arbitrarily to a set of elements a given GPU happens to favor.

What I care about is vector graphics and software freedom. There are some promising techniques to support rasterizing
vector graphics on the GPU, but these techniques are a little too bleeding-edge for a project like this.

### Why Rust?

 * Performance
 * Safety

This software needs to be stasble, and C and C++ force you to do things that are fundamentally unsafe. 
The analysis that Rust's type system affords *out of the box* is phenominal coming from C and C++.

Dependency management is another up-side. Rust's `cargo` is on par with `npm`, `maven`,
more friendly than `sbt`, and superior to `pip`. C++ *might* get modules in 2020.

The downsides (nothing is perfect):
 - Compilation times are slow on x86. And verrrry slow on my target arm system.
 - The compiler is stupid and pedantic and will make you very angry. That's the whole point.
 - Library support (as opposed to the package manager itself) is still patchy (though it's rapidly improving).
 - Stable Rust is still too bleeding-edge for most distributions, and even for meta-distros like buildroot.
  
# Why? Just... Why?

I have a couple of track-only vehicles (race cars). Race cars don't come with dashboards,
you have to install one. I have used or investigated a wide range of electronic dashboards,
all proprietary. Tinkering is a big part of racing. Sometimes you gotta do things your own way
because you can't stand not to.
