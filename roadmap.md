## Roadmap

*update: 2021-2-28*

The master branch hasn't been touched in a while, but is functional. It may or may not not build on newer rust. Some libraries required nightly as of may 2019. I would like to switch back to stable rust as soon as dependencies allow.

Most of the recent work is on the `vm_render` branch, but it's a bit of a mess. That branch is going to be removed soon, and the VM-related aspects will be ported over to the uDLang project. The work un-related to the custom virtual machine will eventually land on master, and the result will be tagged as version 0.1-mvp.

The majority of my effort will be directed towards uDLang, at least until it is featureful enough to host uDashboard.

For more details, see `roadmap.md`.

### 0.1-mvp

A cleaned-up version of the existing code. I have learned more about Rust since I started this, and some of the language constraints have been relaxed on stable rust. So, I think I know enough now to clean up the somewhat convoluted code in the rendering layer.`XXX` / `FIXME` / `TODO` issues now.

In addition to the features currently on master, ther will also be:
- a windowed mode for debugging configurations on the desktop.
- A simple graphical tool for testing and debugging. Some text-based scripts to work with data are in the scripts directory. These will be left, but a graphical tool will be preferable in most cases.

### 0.2-stable

This will be a minor point release, if it happens. Bug fixes and minor improvements. Code re-organization and clearing out dead code / misleading documentation, etc.

- buildroot integration at least written and hosted somewhere. will attempt to upstream.

### 1.0-mvp

Demonstration / proof of concept release, with minimum buildroot config for raspberry pi hardware. This will be a massive rewrite. Once uDLang hits 0.1-mvp, uDashboard will be ported over to it.

- The existing rendering codebase will become a thin wrapper around cairo.
- A uDLang companion library will be written.
- Will include a small library of Gauges, layouts, example configs. 
  - all be expressed in uDLang, written against the companion library.
- This will be benchmarked and evaluated against 0.1-mvp.

If this effort doesn't pan out, I reserve the right to abandon it, and continue evolving the pre-1.0 codebase.

### 1.1 - 1.9

Subsequent releases series will focus on tooling.

- Configuration tool
- Wysiwyg Editors
- Web config
- Authoring tools
- Buildroot / Yocto / NixOS / Debian packaging
- Repository of user-contributed content (layouts, instrumentation designs, etc).

### 2.0

Focus on GPU acceleration.
