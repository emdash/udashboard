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
use crate::ast::{CairoOp};
use crate::config::Screen;
use crate::data::State;
use crate::vm::VM;
use crate::vm::Env;
use crate::vm::Output;
use crate::vm::Program;
use crate::vm::Result;
use crate::vm::Value;


use std::cell::RefCell;
use std::fs;

use cairo;
use cairo::{Context, Format, ImageSurface};



// TODO: promote to env var or cli param ideally this is derived from
// the input program, up to some reasonable maximm limit determined by
// available ram.
const STACK_DEPTH: usize = 1024;


pub struct CairoRenderer {
    pub screen: Screen,
    vm: RefCell<VM>
}


struct Hack<'a> {
    cr: &'a Context
}


impl<'a> Hack<'a> {

    fn set_source_rgb(&self, vm: &mut VM) -> Result<()> {
        let b: f64 = vm.pop_into()?;
        let g: f64 = vm.pop_into()?;
        let r: f64 = vm.pop_into()?;
        println!("rgb({:?}, {:?}, {:?})", r, g, b);
        self.cr.set_source_rgb(r, g, b);
        Ok(())
    }

    fn set_source_rgba(&self, vm: &mut VM) -> Result<()> {
        let a: f64 = vm.pop_into()?;
        let g: f64 = vm.pop_into()?;
        let b: f64 = vm.pop_into()?;
        let r: f64 = vm.pop_into()?;
        println!("rgba({:?}, {:?}, {:?}, {:?})", r, g, b, a);
        self.cr.set_source_rgba(r, g, b, a);
        Ok(())
    }

    fn rect(&self, vm: &mut VM) -> Result<()> {
        let h: f64 = vm.pop_into()?;
        let w: f64 = vm.pop_into()?;
        let y: f64 = vm.pop_into()?;
        let x: f64 = vm.pop_into()?;
        println!("rect({:?}, {:?}, {:?}, {:?})", x, y, w, h);
        self.cr.rectangle(x, y, w, h);
        Ok(())
    }

    fn fill(&self) -> Result<()> {
        println!("fill");
        self.cr.fill();
        Ok(())
    }

    fn stroke(&self) -> Result<()> {
        println!("stroke");
        self.cr.stroke();
        Ok(())
    }

    fn paint(&self) -> Result<()> {
        println!("paint");
        self.cr.paint();
        Ok(())
    }
}


impl<'a> Output for Hack<'a> {
    fn output(&mut self, op: CairoOp, vm: &mut VM) -> Result<()> {
        use CairoOp::*;
        match op {
            SetSourceRgb => self.set_source_rgb(vm),
            SetSourceRgba => self.set_source_rgba(vm),
            Rect => self.rect(vm),
            Fill => self.fill(),
            Stroke => self.stroke(),
            Paint => self.paint()
        }
    }
}


impl CairoRenderer {
    pub fn new(
        screen: Screen,
        program: Program
    ) -> CairoRenderer {
        let vm = RefCell::new(VM::new(program, STACK_DEPTH));
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
        cr.identity_matrix();
        let mut hack = Hack { cr };

        let env: Env = state
            .values
            .iter()
            .map(|item| (item.0.clone(), Value::Float(*item.1)))
            .collect();

        // TODO: do something useful with result
        let _ = self.vm.borrow_mut().exec(&env, &mut hack);
    }
}


pub struct PNGRenderer {
    renderer: CairoRenderer,
    path: String,
}

impl PNGRenderer {
    pub fn new(
        path: String,
        renderer: CairoRenderer
    ) -> PNGRenderer {
        PNGRenderer {renderer, path}
    }

    pub fn render(&self, state: &State) {
        let surface = ImageSurface::create(
            Format::ARgb32,
            self.renderer.screen.width as i32,
            self.renderer.screen.height as i32
        ).expect("Couldn't create surface.");
        let cr = Context::new(&surface);

        self.renderer.render(&cr, state);
        let mut file = fs::File::create(self.path.clone())
            .expect("couldn't create file");
        surface.write_to_png(&mut file).unwrap();
    }
}
