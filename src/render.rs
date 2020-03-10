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

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;

use cairo;
use cairo::{Context, Format, ImageSurface};
use regex::Regex;



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


pub enum Insn {
    Op(Opcode<CairoOperation>),
    Val(Value)
}


// XXX: this function is just a place-holder until I get parsing
// working via some other mechanism, for example serde, or syn.
pub fn decode_word(word: &str) -> Option<Insn> {
    lazy_static! {
        static ref STR_REGEX: Regex = Regex::new(
            "\"([^\"]*)\""
        ).unwrap();
    }

    if word.starts_with("#") {
        if let Ok(x) = word[1..].parse::<usize>() {
            Some(Insn::Val(Value::Addr(x)))
        } else {
            None
        }
    } else if let Some(captures) = STR_REGEX.captures(word) {
        let raw = captures.get(1).unwrap().as_str();
        Some(Insn::Val(Value::Str(Rc::new(String::from(raw)))))
    } else if let Ok(x) = word.parse::<f64>() {
        Some(Insn::Val(Value::Float(x)))
    } else if let Ok(x) = word.parse::<i64>() {
        Some(Insn::Val(Value::Int(x)))
    } else if let Ok(x) = word.parse() {
        Some(Insn::Val(Value::Bool(x)))
    } else {
        use Insn::*;
        use Opcode::*;
        use CairoOperation::*;
        match word {
            "load" => Some(Op(Load)),
            "get" => Some(Op(Get)),
            "bool" => Some(Op(Coerce(TypeTag::Bool))),
            "int" => Some(Op(Coerce(TypeTag::Int))),
            "float" => Some(Op(Coerce(TypeTag::Float))),
            "bt" => Some(Op(BranchTrue)),
            "bf" => Some(Op(BranchTrue)),
            "ba" => Some(Op(Branch)),
            "index" => Some(Op(Index)),
            "." => Some(Op(Dot)),
            "rgb" => Some(Op(Disp(SetSourceRgb))),
            "rgba" => Some(Op(Disp(SetSourceRgba))),
            "rect" => Some(Op(Disp(Rect))),
            "fill" => Some(Op(Disp(Fill))),
            "stroke" => Some(Op(Disp(Stroke))),
            "paint" => Some(Op(Disp(Paint))),
            "!" => Some(Op(Break)),
            _ => None
        }
    }
}


pub type ParseResult<Effect> = std::result::Result<Program<Effect>, String>;


pub fn parse(source: &str) -> ParseResult<CairoOperation> {
    let mut code = Vec::new();
    let mut values: HashMap<&str, usize> = HashMap::new();
    let mut index: usize = 0;
    let mut data = Vec::new();

    for (i, word) in source.split_whitespace().enumerate() {
        match decode_word(&word) {
            Some(Insn::Val(val)) => if let Some(existing) = values.get(word) {
                code.push(Opcode::Push(Immediate::Addr(*existing)));
                code.push(Opcode::Load);
            } else {
                values.insert(word, index);
                data.push(val);
                code.push(Opcode::Push(Immediate::Addr(index)));
                code.push(Opcode::Load);
                index += 1;
            },
            Some(Insn::Op(opcode)) => code.push(opcode),
            None => return Err(String::from(word))
        }
    }

    Ok(Program {code, data})
}


pub fn load(path: String) -> ParseResult<CairoOperation> {
    if let Ok(source) = fs::read_to_string(path) {
        parse(source.as_str())
    } else {
        Err(String::from("Couldn't open file"))
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
        let mut file = fs::File::create(self.path.clone())
            .expect("couldn't create file");
        surface.write_to_png(&mut file).unwrap();
    }
}
