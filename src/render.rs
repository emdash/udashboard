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
use crate::ast::BinOp;
use crate::ast::UnOp;
use crate::config::Screen;
use crate::data::State;
use crate::vm::Env;
use crate::vm::Error;
use crate::vm::Immediate;
use crate::vm::Opcode;
use crate::vm::Output;
use crate::vm::Program;
use crate::vm::Result;
use crate::vm::TypeTag;
use crate::vm::Value;
use crate::vm::VM;

use std::fs::File;
use std::cell::RefCell;
use std::rc::Rc;

use cairo;
use cairo::{Context, Format, ImageSurface};


// TODO: promote to env var or cli param ideally this is derived from
// the input program, up to some reasonable maximm limit determined by
// available ram.
const stack_depth: usize = 1024;


// Enum for context-specific operations
#[derive(Copy, Clone, Debug)]
pub enum CairoOperation {
    SetSourceRgb,
    SetSourceRgba,
    Rect,
    Fill,
    Stroke,
    Paint
}


type CairoVM = VM<CairoOperation>;


pub struct CairoRenderer {
    screen: Screen,
    vm: RefCell<CairoVM>
}


pub fn decode_word(word: &str) -> Option<Opcode<CairoOperation>> {
    if word.starts_with("#") {
        if let Ok(x) = word[1..].parse::<usize>() {
            Some(Opcode::Push(Immediate::Addr(x)))
        } else {
            None
        }
    } else if let Ok(x) = word.parse::<f64>() {
        Some(Opcode::Push(Immediate::Float(x)))
    } else if let Ok(x) = word.parse::<i16>() {
        Some(Opcode::Push(Immediate::Int(x)))
    } else if let Ok(x) = word.parse() {
        Some(Opcode::Push(Immediate::Bool(x)))
    } else {
        match word {
            "load" => Some(Opcode::Load),
            "get" => Some(Opcode::Get),
            "bool" => Some(Opcode::Coerce(TypeTag::Bool)),
            "int" => Some(Opcode::Coerce(TypeTag::Int)),
            "float" => Some(Opcode::Coerce(TypeTag::Float)),
            "bt" => Some(Opcode::BranchTrue),
            "bf" => Some(Opcode::BranchTrue),
            "ba" => Some(Opcode::Branch),
            "index" => Some(Opcode::Index),
            "." => Some(Opcode::Dot),
            "rgb" => Some(Opcode::Disp(CairoOperation::SetSourceRgb)),
            "rgba" => Some(Opcode::Disp(CairoOperation::SetSourceRgba)),
            "rect" => Some(Opcode::Disp(CairoOperation::Rect)),
            "fill" => Some(Opcode::Disp(CairoOperation::Fill)),
            "stroke" => Some(Opcode::Disp(CairoOperation::Stroke)),
            "paint" => Some(Opcode::Disp(CairoOperation::Paint)),
            "!" => Some(Opcode::Break),
            _ => None
        }
    }
}


pub fn parse(source: &str) -> Program<CairoOperation> {
    let code: Option<Vec<Opcode<CairoOperation>>> = source
        .split_whitespace()
        .map(|word| decode_word(&word.to_owned()))
        .collect();

    Program {
        code: code.unwrap(),
        data: vec! {
            Value::Str(Rc::new(String::from("RPM"))),
            Value::Str(Rc::new(String::from("ECT"))),
            Value::Str(Rc::new(String::from("OIL_PRESSURE")))
        }
    }
}


struct Hack<'a> {
    cr: &'a Context
}


impl<'a> Hack<'a> {

    fn set_source_rgb(&self, vm: &mut CairoVM) -> Result<()> {
        let g: f64 = vm.pop_into()?;
        let b: f64 = vm.pop_into()?;
        let r: f64 = vm.pop_into()?;
        println!("{:?}, {:?}, {:?}", r, g, b);
        self.cr.set_source_rgb(r, g, b);
        Ok(())
    }

    fn set_source_rgba(&self, vm: &mut CairoVM) -> Result<()> {
        let a: f64 = vm.pop_into()?;
        let g: f64 = vm.pop_into()?;
        let b: f64 = vm.pop_into()?;
        let r: f64 = vm.pop_into()?;
        self.cr.set_source_rgba(r, g, b, a);
        Ok(())
    }

    fn rect(&self, vm: &mut CairoVM) -> Result<()> {
        let h: f64 = vm.pop_into()?;
        let w: f64 = vm.pop_into()?;
        let y: f64 = vm.pop_into()?;
        let x: f64 = vm.pop_into()?;
        self.cr.rectangle(x, y, w, h);
        Ok(())
    }

    fn fill(&self) -> Result<()> {
        self.cr.fill();
        Ok(())
    }

    fn stroke(&self) -> Result<()> {
        self.cr.stroke();
        Ok(())
    }

    fn paint(&self) -> Result<()> {
        self.cr.paint();
        Ok(())
    }
}


impl<'a> Output<CairoOperation> for Hack<'a> {
    fn output(&mut self, op: CairoOperation, vm: &mut CairoVM) -> Result<()> {
        use CairoOperation::*;
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
        program: Program<CairoOperation>
    ) -> CairoRenderer {
        let vm = RefCell::new(VM::new(program, stack_depth));
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
