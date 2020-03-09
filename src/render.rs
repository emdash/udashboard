// uDashBoard: featherweight dashboard application.
//
// Copyright (C) 2019  Brandon Lewis
//
// This program is free software: you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public License
// as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public
// License along with this program.  If not, see
// <https://www.gnu.org/licenses/>.

// Cairo rendering implementation
use crate::config::Screen;
use crate::data::State;
use crate::vm;

use std::fs::File;
use std::cell::RefCell;

use cairo;
use cairo::{Context, Format, ImageSurface};


// TODO: promote to env var or cli param ideally this is derived from
// the input program, up to some reasonable maximm limit determined by
// available ram.
const stack_depth: usize = 1024;


pub struct CairoRenderer {
    // XXX: it sucks that need this overhead. The renderer either
    // needs interior mutability for the vm, or else we have to do the
    // rendering outside the paint method.
    //
    // For drm, it doesn't matter, since we control the draw loop.
    // For Gtk, gtk-rs closures are not FnMut, so we can't capture
    // mutable values in them! what's the goddamn point of that?
    screen: Screen,
    vm: RefCell<vm::VM>
}


impl CairoRenderer {
    pub fn new(
        screen: Screen,
        program: vm::Program
    ) -> CairoRenderer {
        let vm = RefCell::new(vm::VM::new(program, stack_depth));
        CairoRenderer { screen, vm }
    }

    pub fn render(
        &self,
        cr: &Context,
        state: &State
    ) {
        // XXX: specify this somewher.
        cr.set_source_rgb(0.0, 0.0, 0.0);
        cr.paint();

        let env: vm::Env = state
            .values
            .iter()
            .map(|item| (item.0.clone(), vm::Value::Float(*item.1)))
            .collect();

        // TODO: do something useful with result
        let _ = self.vm.borrow_mut().exec(&env, &mut ());
    }
}


pub struct PNGRenderer {
    renderer: CairoRenderer,
    path: String,
    screen: Screen
}

impl PNGRenderer {
    pub fn new(
        path: String,
        screen: Screen,
        renderer: CairoRenderer
    ) -> PNGRenderer {
        PNGRenderer {renderer, path, screen}
    }

    pub fn render(&self, state: &State) {
        let surface = ImageSurface::create(
            Format::ARgb32,
            self.screen.width as i32,
            self.screen.height as i32
        ).expect("Couldn't create surface.");
        let cr = Context::new(&surface);

        self.renderer.render(&cr, state);
        let mut file = File::create(self.path.clone())
            .expect("couldn't create file");
        surface.write_to_png(&mut file).unwrap();
    }
}
