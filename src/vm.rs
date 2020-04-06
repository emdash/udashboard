// (C) 2020 Brandon Lewis
//
// A virtual machine for a custom Elm-inspired graphics system.
//
// This system is optimized for short-running kernels that render a
// single frame of video.
//
// The ISA intentionally limits the runtime behavior of the system,
// with the goal of improving both security and performance as
// compared with existing alternatives.
//
// *Execution Model*
//
// A program consists of a sequence of instructions and an immutable
// data section. Almost all instructions mutate or inspect the stack,
// except for the Disp instruction, may not depending on the sub-operation.
//
// Execution is done with respect to an external environment, which is
// a key-value map.
//
// *Validity*
//
// The set of runtime errors is represented by the Error enum in this
// file. All are non-recoverable, modulo an external debugger.
//
// A valid program is one which terminates with Error::Halt.
//
// *Safety*
//
// The main goal is to avoid accidental "weird machines". The
// instruction-set is strongly-typed. Types and other bounds are
// checked at run-time.
//
// *Instructions*
//
// The core instruction set is broadly similar to other
// stack-machines. The usual family of arithmetic, logic, relational,
// and stack manipulation operators are present.
//
// Subroutines are supported with the "call", "ret", and "arg"
// instructions. "call" and "ret" handle the return address and stack
// frame, while "arg" allows stable indexing of function
// parameters.
//
// Control flow is provided by "bt", "bf", and "ba" instructions,
// which take an address as branch taret. Addresses are _logical_,
// i.e. they are an index into the instruction stream, rather than a
// byte address.
//
// *Values*
//
// - int, float, char, string, id, list, map, effect, addr (see below).
//
// Arithmetic is allowed only on int and float types. There is no
// silent coercion.
//
// String, list, and map types are immutable.
//
// List and map types suport indexing and iteration.
//
// The Addr type can only be used with branch instructions, and
// operations on addresses are not supported.
//
// *The Stack*
//
// The stack is variable-length sequence of opaque "cells", which can
// contain aribtrary VM values.
//
// *The Environment*
//
// The environment is a read-only, structured data store, containing
// arbitrary tree-like data that is inter-convertible with JSON. The
// in-memory representation is opaque, to allow for future
// optimization.  The instruction set contains a family of oppcodes
// that allow traversing arbitrary paths through the environment.
//
// *Effects*
//
// The CairoOp type represents the set of valid canvas operations.


use crate::ast::{BinOp, UnOp, CairoOp};
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
use enumflags2::BitFlags;
use regex::Regex;
use std::fs;


// The in-memory opcode format.
//
// This is designed to make illegal operations impossible to
// represent, thereby avoiding "wierd machines" resulting from
// ill-formed opcodes.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Opcode {
    LoadI(u16),
    Load,
    Get,
    Coerce(TypeTag),
    Binary(BinOp),
    Unary(UnOp),
    Call(u8),   // Arity of function arguments
    Ret(u8),    // Arity of function return values
    BranchTrue,
    BranchFalse,
    Branch,
    Drop(u8),
    Dup(u8),    // If you need more than 255 copies, something is wrong.
    Arg(u8),
    Index,
    Dot,
    Expect(TypeTag),
    Disp(CairoOp),
    Break,
    Halt
}


// The result of any operation
pub type Result<T> = core::result::Result<T, Error>;


// All valid values
#[derive(Clone, Debug)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<String>),
    List(Rc<Vec<Value>>),
    Map(Rc<Env>),
    Addr(usize),
}


// It kinda bugs me that I need this, but Rust doesn't have a way of
// exposing an enum's discriminant besides a pattern match.
#[derive(BitFlags, Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum TypeTag {
    Bool  = 0b0000001,
    Int   = 0b0000010,
    Float = 0b0000100,
    Str   = 0b0001000,
    List  = 0b0010000,
    Map   = 0b0100000,
    Addr  = 0b1000000
}


type TypeSet = BitFlags<TypeTag>;


// Like core::Into, except that it returns a Result.
//
// The problem with Into is that Into<T>::into() returns a T, and
// since this is a runtime value, we need to implement Into for
// Result<T>, not plain T, since it can fail at runtime. The compiler
// isn't smart enough to deduce the type.
pub trait TryInto<T> {
    fn try_into(self) -> Result<T>;
}


// Construct an Error::TypeError from a value.
fn expected(expect: TypeSet, got: &Value) -> Error {
    Error::TypeError { expect, got: got.get_type() }
}

// Construct an Error::TypeMismatch from a value.
fn type_mismatch(a: &Value, b: &Value) -> Error {
    Error::TypeMismatch(a.get_type(), b.get_type())
}


// Factors out the boiler plate in operator method implementations.
//
// There are two matchers: binary and unary.
//
// They both the name of the method to be defined, and a list of
// <pattern> => <expr>, which is the white-list of operands which
// actually implement the operator. Anything not included in the match
// table is implictly a runtime error.
macro_rules! operator {
    // Template for a unary operator
    (un $name:ident ($expect:expr) { $( $p:pat => $e:expr ),+ } ) => {
        pub fn $name (&self) -> Result<Value> {
            // Bringing Value into scope saves us some characters
            use Value::*;
            match self {
                $($p => Ok($e)),+ ,
                value => Err(expected($expect, value))
            }
        }
    };

    // Template for a binary operator
    (bin $name:ident { $( $p:pat => $e:expr ),+ } ) => {
        pub fn $name (&self, other: &Value) -> Result<Value> {
            // Bringing value into scope saves us some characters.
            use Value::*;
            #[allow(unreachable_patterns)]
            match (self, other) {
                $($p => Ok($e)),+ ,
                (a, b) => Err(type_mismatch(a, b))
            }
        }
    };
}


impl Value {
    pub fn coerce(self, tt: TypeTag) -> Result<Value> {
        match (self, tt) {
            (Value::Bool(v),  TypeTag::Bool)  => Ok(Value::Bool(v)),
            (Value::Bool(v),  TypeTag::Int)   => Ok(Value::Int(v as i64)),
            (Value::Int(v),   TypeTag::Bool)  => Ok(Value::Bool(v != 0)),
            (Value::Int(v),   TypeTag::Int)   => Ok(Value::Int(v)),
            (Value::Int(v),   TypeTag::Float) => Ok(Value::Float(v as f64)),
            (Value::Float(v), TypeTag::Int)   => Ok(Value::Int(v as i64)),
            (Value::Float(v), TypeTag::Float) => Ok(Value::Float(v)),
            (Value::Str(v),   TypeTag::Bool)  => Ok(Value::Bool(!v.is_empty())),
            (Value::List(v),  TypeTag::Bool)  => Ok(Value::Bool(!v.is_empty())),
            (Value::Map(v),   TypeTag::Bool)  => Ok(Value::Bool(!v.is_empty())),
            (a,               b)
                => Err(Error::TypeMismatch(a.get_type(), b))
        }
    }

    operator! { un abs (TypeTag::Int | TypeTag::Float) {
        Int(value)   => Value::Int(value.abs()),
        Float(value) => Value::Float(value.abs())
    } }

    operator! { bin pow {
        // XXX: silent coercion to u32.
        (Int(a),   Int(b))   => Value::Int(a.pow(*b as u32)),
        (Float(a), Float(b)) => Value::Float(a.powf(*b))
    } }

    operator! { bin min {
        // XXX: silent coercion to u32.
        (Int(a),   Int(b))   => Value::Int(*a.min(b)),
        (Float(a), Float(b)) => Value::Float(a.min(*b))
    } }

    operator! { bin max {
        // XXX: silent coercion to u32.
        (Int(a),   Int(b))   => Value::Int(*a.max(b)),
        (Float(a), Float(b)) => Value::Float(a.max(*b))
    } }

    operator! { bin add {
        (Int(a),   Int(b))   => Int(a + b),
        (Float(a), Float(b)) => Float(a + b)
    } }

    operator! { bin sub {
        (Int(a),   Int(b))   => Int(a - b),
        (Float(a), Float(b)) => Float(a - b)
    } }

    operator! { bin mul {
        (Int(a),   Int(b))   => Int(a * b),
        (Float(a), Float(b)) => Float(a * b)
    } }

    operator! { bin div {
        (Int(a),   Int(b))   => Int(a / b),
        (Float(a), Float(b)) => Float(a / b)
    } }

    operator! { bin modulo {
        (Int(a),   Int(b))   => Int(a % b),
        (Float(a), Float(b)) => Float(a % b)
    } }

    operator! { bin bitand {
        (Bool(a), Bool(b)) => Bool(a & b),
        (Int(a),  Int(b))  => Int(a & b)
    } }

    operator! { bin bitor {
        (Bool(a), Bool(b)) => Bool(a | b),
        (Int(a),  Int(b))  => Int(a | b)
    } }

    operator! { bin bitxor {
        (Bool(a), Bool(b)) => Bool(a ^ b),
        (Int(a),  Int(b))  => Int(a ^ b)
    } }

    operator! { bin shl { (Int(a), Int(b)) => Int(a << b) } }

    operator! { bin shr { (Int(a), Int(b)) => Int(a >> b) } }

    operator! { un not (TypeTag::Bool | TypeTag::Int) {
        Bool(a) => Bool(!a),
        Int(a) => Int(!a)
    } }

    operator! { un neg (TypeTag::Int | TypeTag::Float) {
        Int(a) => Int(-a),
        Float(a) => Float(-a)
    } }

    operator! { bin lt {
        (Int(a), Int(b)) => Bool(a < b),
        (Float(a), Float(b)) => Bool(a < b)
    } }

    operator! { bin gt {
        (Int(a), Int(b)) => Bool(a > b),
        (Float(a), Float(b)) => Bool(a > b)
    } }

    operator! { bin lte {
        (Int(a), Int(b)) => Bool(a <= b),
        (Float(a), Float(b)) => Bool(a <= b)
    } }

    operator! { bin gte {
        (Int(a), Int(b)) => Bool(a >= b),
        (Float(a), Float(b)) => Bool(a >= b)
    } }

    operator! { bin eq {
        (Bool(a),  Bool(b))  => Bool(a == b),
        (Int(a),   Int(b))   => Bool(a == b),
        (Float(a), Float(b)) => Bool(a == b),
        (Str(a),   Str(b))   => Bool(a == b),
        (List(a),  List(b))  => Bool(a == b),
        (Map(a),   Map(b))   => Bool(a == b),
        (Addr(a),  Addr(b))  => Bool(a == b),
        // Evaluate to false on type mismatch
        (_,        _)        => Bool(false)
    } }

    pub fn get_type(&self) -> TypeTag {
        match &self {
            Value::Bool(_)  => TypeTag::Bool,
            Value::Int(_)   => TypeTag::Int,
            Value::Float(_) => TypeTag::Float,
            Value::Str(_)   => TypeTag::Str,
            Value::List(_)  => TypeTag::List,
            Value::Map(_)   => TypeTag::Map,
            Value::Addr(_)  => TypeTag::Addr,
        }
    }
}


// Factor out boilerplate for implementation of TryInto
macro_rules! impl_try_into {
    ($variant:ident => $type:ty) => {
        impl TryInto<$type> for Value {
            fn try_into(self) -> Result<$type> {
                match self {
                    Value::$variant(value) => Ok(value.clone()),
                    v => Err(expected(BitFlags::from_flag(TypeTag::$variant), &v))
                }
            }
        }
    }
}

impl_try_into! { Bool  => bool }
impl_try_into! { Int   => i64 }
impl_try_into! { Float => f64 }
impl_try_into! { Str   => Rc<String> }
impl_try_into! { List  => Rc<Vec<Value>> }
impl_try_into! { Map   => Rc<Env> }
impl_try_into! { Addr  => usize }


impl PartialEq for Value {
    fn eq(&self, rhs: &Self) -> bool {
        match Value::eq(self, rhs) {
            Ok(Value::Bool(x)) => x,
            x => panic!("Comparison failed: {:?}", x)
        }
    }
}


/******************************************************************************/

// This is another crucial value type, especially because it's
// propagated up the stack.
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Underflow,
    Overflow,
    NotImplemented,
    IllegalOpcode,
    IllegalAddr(usize),
    TypeError {
        expect: TypeSet,
        got: TypeTag
    },
    TypeMismatch(TypeTag, TypeTag),
    //NameError(Rc<String>),
    IndexError(usize),
    KeyError(String),
    Arity(u8, u8),
    DebugBreak,
    Halt,
}


type Stack = Vec<Value>;
pub type Env = HashMap<String, Value>;


// The internal program representation.
#[derive(Clone, Debug)]
pub struct Program {
    pub code: Vec<Opcode>,
    pub data: Vec<Value>
}


// The external program representation
#[derive(Clone, Debug, PartialEq)]
pub enum Insn where {
    Op(Opcode),
    Label(String),
    LabelRef(String),
    Val(Value),
}


// XXX: this function is just a place-holder until I get parsing
// working via some other mechanism, for example serde, or syn.
pub fn decode_word(word: &str) -> Option<Insn> {
    lazy_static! {
        static ref STR_REGEX: Regex = Regex::new(
            "\"([^\"]*)\""
        ).unwrap();
    }

    lazy_static! {
        static ref LABEL_REGEX: Regex = Regex::new(
            "([a-zA-Z0-9_-]+):"
        ).unwrap();
    }

    println!("{:?}", word);

    if word.starts_with("#") {
        Some(Insn::LabelRef(String::from(&word[1..])))
    } else if word.starts_with("drop:") {
        if let Ok(n) = word[5..].parse::<u8>() {
            Some(Insn::Op(Opcode::Drop(n)))
        } else {
            None
        }
    } else if word.starts_with("dup:") {
        if let Ok(n) = word[4..].parse::<u8>() {
            Some(Insn::Op(Opcode::Dup(n)))
        } else {
            None
        }
    } else if word.starts_with("arg:") {
        if let Ok(n) = word[4..].parse::<u8>() {
            Some(Insn::Op(Opcode::Arg(n)))
        } else {
            None
        }
    } else if word.starts_with("call:") {
        if let Ok(n) = word[5..].parse::<u8>() {
            Some(Insn::Op(Opcode::Call(n)))
        } else {
            None
        }
    } else if word.starts_with("ret:") {
        if let Ok(n) = word[4..].parse::<u8>() {
            Some(Insn::Op(Opcode::Ret(n)))
        } else {
            None
        }
    } else if let Some(captures) = STR_REGEX.captures(word) {
        let raw = captures.get(1).unwrap().as_str();
        Some(Insn::Val(Value::Str(Rc::new(String::from(raw)))))
    } else if let Some(captures) = LABEL_REGEX.captures(word) {
        let raw = captures.get(1).unwrap().as_str();
        Some(Insn::Label(String::from(raw)))
    } else if let Ok(x) = word.parse::<i64>() {
        Some(Insn::Val(Value::Int(x)))
    } else if let Ok(x) = word.parse::<f64>() {
        Some(Insn::Val(Value::Float(x)))
    } else if let Ok(x) = word.parse() {
        Some(Insn::Val(Value::Bool(x)))
    } else {
        use Insn::*;
        use Opcode::*;
        use CairoOp::*;
        match word {
            "load" => Some(Op(Load)),
            "get" => Some(Op(Get)),
            "bool" => Some(Op(Coerce(TypeTag::Bool))),
            "int" => Some(Op(Coerce(TypeTag::Int))),
            "float" => Some(Op(Coerce(TypeTag::Float))),
            "+" => Some(Op(Binary(BinOp::Add))),
            "-" => Some(Op(Binary(BinOp::Sub))),
            "*" => Some(Op(Binary(BinOp::Mul))),
            "/" => Some(Op(Binary(BinOp::Div))),
            "%" => Some(Op(Binary(BinOp::Mod))),
            "**" => Some(Op(Binary(BinOp::Pow))),
            "and" => Some(Op(Binary(BinOp::And))),
            "or" => Some(Op(Binary(BinOp::Or))),
            "xor" => Some(Op(Binary(BinOp::Xor))),
            "<" => Some(Op(Binary(BinOp::Lt))),
            ">" => Some(Op(Binary(BinOp::Gt))),
            ">=" => Some(Op(Binary(BinOp::Gte))),
            "<=" => Some(Op(Binary(BinOp::Lte))),
            "==" => Some(Op(Binary(BinOp::Eq))),
            "<<" => Some(Op(Binary(BinOp::Shl))),
            ">>" => Some(Op(Binary(BinOp::Shr))),
            "min" => Some(Op(Binary(BinOp::Min))),
            "max" => Some(Op(Binary(BinOp::Max))),
            "not" => Some(Op(Unary(UnOp::Not))),
            "neg" => Some(Op(Unary(UnOp::Neg))),
            "abs" => Some(Op(Unary(UnOp::Abs))),
            "bt" => Some(Op(BranchTrue)),
            "bf" => Some(Op(BranchFalse)),
            "ba" => Some(Op(Branch)),
            "index" => Some(Op(Index)),
            "." => Some(Op(Dot)),
            "rgb" => Some(Op(Disp(SetSourceRgb))),
            "rgba" => Some(Op(Disp(SetSourceRgba))),
            "rect" => Some(Op(Disp(Rect))),
            "fill" => Some(Op(Disp(Fill))),
            "stroke" => Some(Op(Disp(Stroke))),
            "paint" => Some(Op(Disp(Paint))),
            "break" => Some(Op(Break)),
            "halt" => Some(Op(Halt)),
            _ => None
        }
    }
}


pub fn load(path: String) -> ParseResult {
    if let Ok(source) = fs::read_to_string(path) {
        let insns: Option<Vec<Insn>> = source
                          .as_str()
                          .split_whitespace()
                          .map(decode_word)
                          .collect();

        match insns {
            Some(insns) => lower(insns),
            None => Err(String::from(
                "Illegal operation somewhere, g.l. finding it."
            ))
        }
    } else {
        Err(String::from("Couldn't open file"))
    }
}


pub type ParseResult = std::result::Result<Program, String>;


// Convert labels to addresses.
pub fn filter_labels(insns: Vec<Insn>) -> Vec<Insn> {
    let mut with_labels_removed = Vec::new();
    let mut labels = HashMap::new();
    for i in insns {
        match i {
            Insn::Label(name) => {
                let index = with_labels_removed.len() as usize;
                let op = Insn::Val(Value::Addr(index));
                labels.insert(name, op);
            },
            insn => with_labels_removed.push(insn)
        }
    }

    println!("{:?}", labels);

    let mut ret = Vec::new();
    for i in with_labels_removed {
        match i {
            Insn::LabelRef(name) => ret.push(
                labels
                    .get(&name)
                    .expect(&("name error: ".to_owned() + &name))
                    .clone()
            ),
            insn => ret.push(insn),
        }
    }

    ret
}


// Lower the external representation to the internal one.
pub fn lower(insns: Vec<Insn>) -> ParseResult
{
    let mut values: HashMap<String, u16> = HashMap::new();
    let mut data = Vec::new();
    let mut code = Vec::new();

    // Convert immediate values to LoadI from a data cell.
    for i in filter_labels(insns) {
        // XXX: Temporary hack to work around the fact that f64
        // doesn't implement hash apis. Equivalent values should
        // stringify to the same string.
        let str_repr = format!("{:?}", i);
        match i {
            Insn::Val(val) => if let Some(existing) = values.get(&str_repr) {
                code.push(Opcode::LoadI(*existing));
            } else {
                // XXX: check len < 64k
                let index = data.len() as u16;
                values.insert(str_repr, index);
                data.push(val);
                code.push(Opcode::LoadI(index));
            },
            Insn::Op(opcode) => code.push(opcode),
            Insn::Label(_) => panic!("Labels should have been resolved."),
            Insn::LabelRef(_) => panic!("Labels should have been resolved.")
        }
    }

    for (i, ii) in code.iter().enumerate() {
        println!("{:?} {:?}", i, ii);
    }

    Ok(Program {code, data})
}


impl Program {
    // Safely fetch the opcode from the given address.
    //
    // The address is simply the index into the instruction sequence.
    fn fetch(&self, index: usize) -> Result<Opcode> {
        let len = self.code.len();

        if index < len {
            Ok(self.code[index])
        } else if index == len {
            Err(Error::Halt)
        } else {
            Err(Error::IllegalAddr(index))
        }
    }

    // Safely retrieve the global static data from the given address.
    //
    // The address is simply the index into the data section.
    pub fn load(&self, index: usize) -> Result<Value> {
        if index < self.data.len() {
            Ok(self.data[index].clone())
        } else {
            Err(Error::IllegalAddr(index))
        }
    }
}


#[derive(Copy, Clone)]
struct StackFrame {
    return_address: usize,
    frame_pointer: usize,
    arity: u8
}


// The entire VM state.
pub struct VM {
    program: Program,
    stack: Stack,
    call_stack: Vec<StackFrame>,
    cur_frame: StackFrame,
    pc: usize
}


// The type of control flow an instruction can have.
pub enum ControlFlow {
    Advance,
    Branch(usize),
    Yield(Value),
}


// trait for capturing VM debug output (result of Disp opcode)
pub trait Output {
    fn output(&mut self, ef: CairoOp, vm: &mut VM) -> Result<()>;
}


// Somewhat naive implementation. Not optimal, but hopefully safe.
//
// TODO: Store borrow of Env internally, so we an make `step` safe,
// and then implement `Iterator`.
//
// TODO: Implement in-place stack mutation, and benchmark to see if it
// offers any improvement.
//
// TODO: Trap mechanism for non-fatal errors. Default to fatal if no
// handler registered.
//
// TODO: Handle integer overflow, and FP NaN as traps, so user code
// can deal.
impl VM {
    pub fn new(program: Program, depth: usize) -> VM {
        VM {
            program: program,
            stack: Stack::with_capacity(depth),
            call_stack: Vec::new(),
            cur_frame: StackFrame {
                return_address: 0,
                frame_pointer: 0,
                arity: 0
            },
            pc: 0,
        }
    }

    // Helper method for poping from stack and type-checking the result.
    pub fn pop(&mut self) -> Result<Value> {
        if let Some(value) = self.stack.pop() {
            Ok(value)
        } else {
            Err(Error::Underflow)
        }
    }

    // Return the current stack depth.
    pub fn depth(&self) -> usize { self.stack.len() }

    // Run the entire program until it halts.
    pub fn exec(
        &mut self,
        env: &Env,
        out: &mut impl Output
    ) -> Result<()> {
        trace!("{:?}", &self.program);
        self.pc = 0;
        self.stack.clear();
        self.call_stack.clear();
        self.cur_frame = StackFrame {
            return_address: 0,
            frame_pointer: 0,
            arity: 0
        };
        // Safe, because we have borrowed env and so by contract it
        // is immutable.
        loop { unsafe {
            match self.step(env, out) {
                Err(Error::Halt) => return Ok(()),
                Err(x) => return Err(x),
                Ok(_) => continue
            }
        } }
     }

    // Single-step the program.
    //
    // Note, this API is intended mainly as an interface for an
    // external debugger, but it is unsafe in the following way: It
    // allows the embedding code to mutate `env` between VM cycles,
    // violating the assumption that `env` is immutable, and
    // potentially leading to undefined behavior. You have been
    // warned.
    //
    // Also, this API is unstable. I will make it safe to use when I
    // can get my head around lifetime parameters in struct
    // definitions, but I am having a hard enough time with the
    // type-checking as it is.
    pub unsafe fn step(
        &mut self,
        env: &Env,
        out: &mut impl Output
    ) -> Result<()> {
        let opcode = self.program.fetch(self.pc)?;

        // TODO: if (trace) {
        println!("{:?} {:?} {:?}", self.pc, opcode, self.stack);

        let result = self.dispatch(opcode, env, out)?;

        match result {
            ControlFlow::Advance      => {self.pc += 1;},
            ControlFlow::Branch(addr) => {self.pc = addr;},
            ControlFlow::Yield(v)     => {self.push(v)?; self.pc += 1;},
        };

        Ok(())
    }

    // Push value onto stack
    pub fn push(&mut self, v: Value) -> Result<ControlFlow> {
        if self.stack.len() < self.stack.capacity() {
            self.stack.push(v);
            Ok(ControlFlow::Advance)
        } else {
            Err(Error::Overflow)
        }
    }

    // Load from constant data section.
    pub fn load(&mut self) -> Result<ControlFlow> {
        match self.pop() {
            Ok(Value::Addr(address)) => self.load_immediate(address),
            Ok(v) => Err(expected(BitFlags::from_flag(TypeTag::Addr), &v)),
            Err(e) => Err(e)
        }
    }

    pub fn load_immediate(&mut self, addr: usize) -> Result<ControlFlow> {
        self.push(self.program.load(addr)?)?;
        Ok(ControlFlow::Advance)
    }

    // Return element from the environment map.
    fn get(&mut self, env: &Env) -> Result<ControlFlow> {
        let key: Rc<String> = self.pop_into()?;
        let key = key.to_string();
        if let Some(value) = env.get(&key) {
            Ok(ControlFlow::Yield(value.clone()))
        } else {
            Err(Error::KeyError(key))
        }
    }

    // Dispatch opcode to the Value implementation.
    fn binop(&mut self, op: BinOp) -> Result<ControlFlow> {
        let bb = self.pop()?;
        let b = &bb;
        let a = self.pop()?;
        let ret = match op {
            BinOp::Add  => a.add(b),
            BinOp::Sub  => a.sub(b),
            BinOp::Mul  => a.mul(b),
            BinOp::Div  => a.div(b),
            BinOp::Mod  => a.modulo(b),
            BinOp::Pow  => a.pow(b),
            BinOp::And  => a.bitand(b),
            BinOp::Or   => a.bitor(b),
            BinOp::Xor  => a.bitxor(b),
            BinOp::Lt   => a.lt(b),
            BinOp::Gt   => a.gt(b),
            BinOp::Lte  => a.lte(b),
            BinOp::Gte  => a.gte(b),
            BinOp::Eq   => a.eq(b),
            BinOp::Shl  => a.shl(b),
            BinOp::Shr  => a.shr(b),
            BinOp::Min  => a.min(b),
            BinOp::Max  => a.max(b)
        }?;
        Ok(ControlFlow::Yield(ret))
    }

    // Dispatch opcode to Value implementation.
    fn unop(&mut self, op: UnOp) -> Result<ControlFlow> {
        let value = self.pop()?;
        Ok(ControlFlow::Yield(match op {
            UnOp::Not  => value.not(),
            UnOp::Neg  => value.neg(),
            UnOp::Abs  => value.abs()
        }?))
    }

    fn coerce(&mut self, tt: TypeTag) -> Result<ControlFlow> {
        Ok(ControlFlow::Yield(self.pop()?.coerce(tt)?))
    }

    // Needed this because type inference failed.
    pub fn pop_into<T>(&mut self) -> Result<T> where Value: TryInto<T> {
        let v: Value = self.pop()?;
        let v: T = v.try_into()?;
        Ok(v)
    }

    // Push frame onto call stack, and branch.
    fn call(&mut self, arity: u8) -> Result<ControlFlow> {
        let target: usize = self.pop_into()?;
        // save frame pointer
        self.call_stack.push(self.cur_frame);
        self.cur_frame.return_address = self.pc + 1;
        self.cur_frame.frame_pointer = self.stack.len() - arity as usize;
        self.cur_frame.arity = arity;
        Ok(ControlFlow::Branch(target))
    }

    // Return from subroutine.
    fn ret(&mut self, retvals: u8) -> Result<ControlFlow> {
        let fp = self.cur_frame.frame_pointer;
        let target = self.cur_frame.return_address;
        for _ in 0..self.cur_frame.arity {
            self.stack.remove(fp);
        }
        assert!(self.stack.len() == fp + retvals as usize);
        self.cur_frame = self.call_stack.pop().unwrap();
        Ok(ControlFlow::Branch(target))
    }

    // Branch if top of stack is true.
    fn branch_true(&mut self) -> Result<ControlFlow> {
        let target: usize = self.pop_into()?;
        let cond: bool = self.pop_into()?;
        Ok(if cond {
            ControlFlow::Branch(target)
        } else {
            ControlFlow::Advance
        })
    }

    // Branch if top of stack is false.
    fn branch_false(&mut self) -> Result<ControlFlow> {
        let target: usize = self.pop_into()?;
        let cond: bool = self.pop_into()?;
        Ok(if cond {
            ControlFlow::Advance
        } else {
            ControlFlow::Branch(target)
        })
    }

    // Unconditional branch
    fn branch(&mut self) -> Result<ControlFlow> {
        let addr: usize = self.pop_into()?;
        Ok(ControlFlow::Branch(addr))
    }

    // Discard top of stack
    fn drop(&mut self, n: u8) -> Result<ControlFlow> {
        for _ in 0..n { self.pop()?; }
        Ok(ControlFlow::Advance)
    }

    // Duplicate top of stack N times.
    fn dup(&mut self, n: u8) -> Result<ControlFlow> {
        let top = self.pop()?;
        for _ in 0..(n + 1) { self.push(top.clone())?; }
        Ok(ControlFlow::Advance)
    }

    // Return argument relative to stack frame
    fn arg(&mut self, n: u8) -> Result<ControlFlow> {
        if n < self.cur_frame.arity {
            let index = self.cur_frame.frame_pointer + n as usize;
            if index < self.stack.len() {
                Ok(ControlFlow::Yield(self.stack[index].clone()))
            } else {
                Err(Error::Underflow)
            }
        } else {
            Err(Error::Arity(n, self.cur_frame.arity))
        }
    }

    // Return element from a list reference
    fn index(&mut self) -> Result<ControlFlow> {
        let index: usize = self.pop_into()?;
        let list: Rc<Vec<Value>> = self.pop_into()?;
        if index < list.len() {
            self.push(list[index].clone())?;
            Ok(ControlFlow::Advance)
        } else {
            Err(Error::IndexError(index))
        }
    }

    // Return element from a map reference
    fn dot(&mut self) -> Result<ControlFlow> {
        let key: Rc<String> = self.pop_into()?;
        let key = key.to_string();
        let map: Rc<Env> = self.pop_into()?;
        if let Some(value) = map.get(&key) {
            self.push(value.clone())?;
            Ok(ControlFlow::Advance)
        } else {
            Err(Error::KeyError(key))
        }
    }

    // Check that top-of-stack matches the given type.
    // Does not consume the stack value.
    fn expect(&self, t: TypeTag) -> Result<ControlFlow> {
        if let Some(value) = self.stack.last() {
            let tt = value.get_type();
            if t == tt {
                Ok(ControlFlow::Advance)
            } else {
                Err(Error::TypeError {
                    expect: TypeSet::from_flag(t),
                    got: tt
                })
            }
        } else {
            Err(Error::Underflow)
        }
    }

    // Emit the top of stack as output.
    fn disp(
        &mut self,
        e: CairoOp,
        out: &mut impl Output
    ) -> Result<ControlFlow> {
        out.output(e, self)?;
        Ok(ControlFlow::Advance)
    }

    // Dispatch table for built-in opcodes
    fn dispatch(
        &mut self,
        op: Opcode,
        env: &Env,
        out: &mut impl Output
    ) -> Result<ControlFlow> {
        match op {
            Opcode::LoadI(addr) => self.load_immediate(addr as usize),
            Opcode::Load        => self.load(),
            Opcode::Get         => self.get(env),
            Opcode::Coerce(t)   => self.coerce(t),
            Opcode::Binary(op)  => self.binop(op),
            Opcode::Unary(op)   => self.unop(op),
            Opcode::Call(arity) => self.call(arity),
            Opcode::Ret(n)      => self.ret(n),
            Opcode::BranchTrue  => self.branch_true(),
            Opcode::BranchFalse => self.branch_false(),
            Opcode::Branch      => self.branch(),
            Opcode::Drop(n)     => self.drop(n),
            Opcode::Dup(n)      => self.dup(n),
            Opcode::Arg(n)      => self.arg(n),
            Opcode::Index       => self.index(),
            Opcode::Dot         => self.dot(),
            Opcode::Expect(t)   => self.expect(t),
            Opcode::Disp(ef)    => self.disp(ef, out),
            Opcode::Break       => Err(Error::DebugBreak),
            Opcode::Halt        => Err(Error::Halt)
        }
    }
}


// These tests are, where possible, written against the *behavior* of
// the VM. *Any* conforming implementation should be able to pass
// these tests.
//
// Any optimizations under consideration should, as a
// criteria for admissibility, at least pass these tests.
//
// As the VM evolves, these tests will constitute part of the
// specification.
#[cfg(test)]
mod tests {
    use super::*;
    use super::BinOp::*;
    use super::UnOp::*;
    use super::Value::*;
    use super::TypeTag as TT;
    use super::Error;
    use super::Rc;
    use super::HashMap;
    use super::Env;
    use std::io::Stdout;

    type VM = super::VM;
    type Program = super::Program;
    use super::Opcode::*;

    impl super::Output for () {
        fn output(&mut self, _: CairoOp, vm: &mut VM) -> Result<()> {
            let _ = vm.pop()?;
            Ok(())
        }
    }

    // Useful for debugging in unit tests.
    impl super::Output for Stdout {
        fn output(&mut self, _: CairoOp, vm: &mut VM) -> Result<()>{
            trace!("{:?}", vm.pop()?);
            Ok(())
        }
    }

    // Used for explicitly testing the effect mechanism.
    impl super::Output for Vec<super::Value> {
        fn output(&mut self, _: CairoOp, vm: &mut VM) -> Result<()>{
            self.push(vm.pop()?);
            Ok(())
        }
    }

    // Shortcut for creating a TypeMismatch error.
    fn tm(a: TypeTag, b: TypeTag) -> Result<Value> {
        Err(Error::TypeMismatch(a, b))
    }

    // Shortcut for creating a TypeError error.
    fn te(expect: TypeSet, got: TypeTag) -> Result<Value> {
        Err(Error::TypeError {expect, got})
    }

    // Shortcut for creating a Str value from literal.
    fn s(v: &'static str) -> Value {
        Str(Rc::new(String::from(v)))
    }

    // Shortcut for creating a List from a slice literal.
    fn l(v: &[Value]) -> Value {
        List(Rc::new(v.to_vec()))
    }

    // Shortcut for creating a Map from a slice literal.
    fn m(v: &[(&'static str, Value)]) -> Value {
        let map = v
            .iter()
            .cloned()
            .map(|item| (String::from(item.0), item.1))
            .collect();

        Map(Rc::new(map))
    }

    // Run program to completion in blank environment.
    //
    // Return the final VM state and status code.
    fn eval(
        stack_limit: usize,
        expected_final_depth: usize,
        prog: Program,
        env: Env
    ) -> Result<Value> {
        let mut vm = VM::new(prog, stack_limit);
        let status = vm.exec(&env, &mut ());

        // Program is assumed to have left result in top-of-stack.
        match status {
            Err(e) => {
                Err(e)
            },
            Ok(()) => {
                assert_eq!(vm.depth(), expected_final_depth);
                vm.pop()
            }
        }
    }

    // Assert that the given program evaluates to the expected result.
    //
    // If expected is Ok(), asserts top of stack is equal to `expected_value`.
    // If expected is Err(), asserts that status =
    fn assert_evaluates_to(
        stack_limit: usize,
        expected_final_depth: usize,
        expected_value: Result<Value>,
        prog: Program,
    ) {
        let env = HashMap::new();
        let result = eval(stack_limit, expected_final_depth, prog, env);
        trace!("assert_evaluates_to: {:?} == {:?})", &expected_value, &result);
        match (result, expected_value) {
            (Ok(r), Ok(e)) => assert_eq!(r, e),
            (Err(r), Err(e)) => assert_eq!(r, e),
            (r, e) => panic!("Assertion failed: {:?} != {:?}", r, e)
        }
    }

    // For testing individual operations, we the expected final stack
    // depth is easy to compute based on the final result.
    fn single_op_depth(value: &Result<Value>) -> usize {
        match value {
            Ok(_) => 1,
            Err(_) => 0
        }
    }

    // Test a unary operation on the given operand.
    fn test_unary(
        op: UnOp,
        value: Value,
        expected: Result<Value>
    ) {
        trace!("test_unary({:?})", op);
        assert_evaluates_to(1, single_op_depth(&expected), expected, Program {
            code: vec! {
                LoadI(0),
                Unary(op)
            },
            data: vec! {value}
        });
    }

    // Test a binary operation on the given operands
    fn test_binary(
        op: BinOp,
        a: Value,
        b: Value,
        expected: Result<Value>
    ) {
        trace!("test_binary({:?})", op);
        assert_evaluates_to(2, single_op_depth(&expected), expected, Program {
            code: vec! {
                LoadI(0),
                LoadI(1),
                Binary(op)
            },
            data: vec! {a, b}
        });
    }

    #[test]
    fn test_simple() {
        let p = Program {
            code: vec! {
                LoadI(0),
                LoadI(1),
                Binary(Add)
            },
            data: vec! {Value::Int(1), Value::Int(2)}
        };

        let mut vm = VM::new(p, 2);
        let env = HashMap::new();
        assert_eq!(vm.exec(&env, &mut ()), Ok(()));

        let result: i64 = vm.pop().unwrap().try_into().unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn test_unary_ops() {
        test_unary(Not, Bool(false), Ok(Bool(true)));
        test_unary(Neg, Bool(true), te(TT::Int | TT::Float, TT::Bool));
        test_unary(Abs, Bool(false), te(TT::Int | TT::Float, TT::Bool));

        test_unary(Not, Int(1), Ok(Int(-2)));
        test_unary(Neg, Int(1), Ok(Int(-1)));
        test_unary(Abs, Int(-1), Ok(Int(1)));

        test_unary(Not, Float(1.0), te(TT::Bool | TT::Int, TT::Float));
        test_unary(Neg, Float(1.0), Ok(Float(-1.0)));
        test_unary(Abs, Float(-1.0), Ok(Float(1.0)));
    }

    #[test]
    fn test_binary_ops() {
        let l1 = l(&[Int(1), Float(2.0), Bool(false)]);
        let l2 = l(&[s("abc"), Addr(1), l(&[])]);
        let m1 = m(&[
            ("foo", Int(1)),
            ("bar", Float(2.0)),
            ("baz", s("quux"))
        ]);
        let m2 = m(&[
            ("foo", Addr(1)),
            ("bar", l1.clone()),
            ("baz", m1.clone())
        ]);


        test_binary(Sub, Bool(false), Bool(false), tm(TT::Bool, TT::Bool));
        test_binary(Mul, Bool(false), Bool(false), tm(TT::Bool, TT::Bool));
        test_binary(Div, Bool(false), Bool(false), tm(TT::Bool, TT::Bool));
        test_binary(Pow, Bool(false), Bool(false), tm(TT::Bool, TT::Bool));
        test_binary(And, Bool(false), Bool(false), Ok(Bool(false)));
        test_binary(And, Bool(false), Bool(true),  Ok(Bool(false)));
        test_binary(And, Bool(true),  Bool(false), Ok(Bool(false)));
        test_binary(And, Bool(true),  Bool(true),  Ok(Bool(true)));
        test_binary(Or,  Bool(false), Bool(false), Ok(Bool(false)));
        test_binary(Or,  Bool(false), Bool(true),  Ok(Bool(true)));
        test_binary(Or,  Bool(true),  Bool(false), Ok(Bool(true)));
        test_binary(Or,  Bool(true),  Bool(true),  Ok(Bool(true)));
        test_binary(Xor, Bool(false), Bool(false), Ok(Bool(false)));
        test_binary(Xor, Bool(false), Bool(true),  Ok(Bool(true)));
        test_binary(Xor, Bool(true),  Bool(false), Ok(Bool(true)));
        test_binary(Xor, Bool(true),  Bool(true),  Ok(Bool(false)));

        test_binary(Lt,  Bool(false), Bool(false), tm(TT::Bool, TT::Bool));
        test_binary(Gt,  Bool(false), Bool(false), tm(TT::Bool, TT::Bool));
        test_binary(Lte, Bool(false), Bool(false), tm(TT::Bool, TT::Bool));
        test_binary(Gte, Bool(false), Bool(false), tm(TT::Bool, TT::Bool));
        test_binary(Eq,  Bool(false), Bool(false), Ok(Bool(true)));
        test_binary(Eq,  Bool(false), Bool(true),  Ok(Bool(false)));
        test_binary(Eq,  Bool(true),  Bool(false), Ok(Bool(false)));
        test_binary(Eq,  Bool(true),  Bool(true),  Ok(Bool(true)));
        test_binary(Shl, Bool(false), Bool(false), tm(TT::Bool, TT::Bool));
        test_binary(Shr, Bool(false), Bool(false), tm(TT::Bool, TT::Bool));
        test_binary(Min, Bool(false), Bool(false), tm(TT::Bool, TT::Bool));
        test_binary(Max, Bool(false), Bool(false), tm(TT::Bool, TT::Bool));

        test_binary(Sub, Int(1), Int(2), Ok(Int(-1)));
        test_binary(Mul, Int(2), Int(3), Ok(Int(6)));
        test_binary(Div, Int(6), Int(2), Ok(Int(3)));
        test_binary(Pow, Int(2), Int(3), Ok(Int(8)));
        test_binary(And, Int(2), Int(3), Ok(Int(2)));
        test_binary(Or,  Int(2), Int(3), Ok(Int(3)));
        test_binary(Xor, Int(2), Int(3), Ok(Int(1)));
        test_binary(Lt,  Int(2), Int(3), Ok(Bool(true)));
        test_binary(Gt,  Int(2), Int(3), Ok(Bool(false)));
        test_binary(Lte, Int(2), Int(2), Ok(Bool(true)));
        test_binary(Gte, Int(2), Int(2), Ok(Bool(true)));
        test_binary(Eq,  Int(2), Int(3), Ok(Bool(false)));
        test_binary(Eq,  Int(2), Int(2), Ok(Bool(true)));
        test_binary(Shl, Int(1), Int(3), Ok(Int(8)));
        test_binary(Shr, Int(8), Int(3), Ok(Int(1)));
        test_binary(Min, Int(2), Int(3), Ok(Int(2)));
        test_binary(Max, Int(2), Int(3), Ok(Int(3)));

        test_binary(Add, Float(1.0), Float(2.0), Ok(Float(3.0)));
        test_binary(Sub, Float(1.0), Float(2.0), Ok(Float(-1.0)));
        test_binary(Mul, Float(2.0), Float(3.0), Ok(Float(6.0)));
        test_binary(Div, Float(6.0), Float(2.0), Ok(Float(3.0)));
        test_binary(Pow, Float(2.0), Float(3.0), Ok(Float(8.0)));
        test_binary(And, Float(2.0), Float(3.0), tm(TT::Float, TT::Float));
        test_binary(Or,  Float(2.0), Float(3.0), tm(TT::Float, TT::Float));
        test_binary(Xor, Float(2.0), Float(3.0), tm(TT::Float, TT::Float));
        test_binary(Lt,  Float(2.0), Float(3.0), Ok(Bool(true)));
        test_binary(Gt,  Float(2.0), Float(3.0), Ok(Bool(false)));
        test_binary(Lte, Float(2.0), Float(2.0), Ok(Bool(true)));
        test_binary(Gte, Float(2.0), Float(2.0), Ok(Bool(true)));
        test_binary(Eq,  Float(2.0), Float(2.0), Ok(Bool(true)));
        test_binary(Eq,  Float(2.0), Float(3.0), Ok(Bool(false)));
        test_binary(Shl, Float(2.0), Float(3.0), tm(TT::Float, TT::Float));
        test_binary(Shr, Float(2.0), Float(3.0), tm(TT::Float, TT::Float));
        test_binary(Min, Float(2.0), Float(3.0), Ok(Float(2.0)));
        test_binary(Max, Float(2.0), Float(3.0), Ok(Float(3.0)));

        test_binary(Eq, l1.clone(), l1.clone(), Ok(Bool(true)));
        test_binary(Eq, l1.clone(), l2.clone(), Ok(Bool(false)));

        test_binary(Eq, m1.clone(), m1.clone(), Ok(Bool(true)));
        test_binary(Eq, m1.clone(), m2.clone(), Ok(Bool(false)));

        // Test For Type Mismatch Errors
        for &op in &[
            Add,
            Sub,
            Mul,
            Div,
            Pow,
            And,
            Or,
            Xor,
            Lt,
            Gt,
            Lte,
            Gte,
            Shl,
            Shr,
            Min,
            Max
        ] {
            test_binary(op, Bool(true), Int(2),     tm(TT::Bool,  TT::Int));
            test_binary(op, Bool(true), Float(2.0), tm(TT::Bool,  TT::Float));
            test_binary(op, Bool(true), s("abc"),   tm(TT::Bool,  TT::Str));
            test_binary(op, Bool(true), l1.clone(), tm(TT::Bool,  TT::List));
            test_binary(op, Bool(true), m1.clone(), tm(TT::Bool,  TT::Map));
            test_binary(op, Bool(true), Addr(3),    tm(TT::Bool,  TT::Addr));
            test_binary(op, Int(1),     Bool(true), tm(TT::Int,   TT::Bool));
            test_binary(op, Int(1),     Float(2.0), tm(TT::Int,   TT::Float));
            test_binary(op, Int(1),     Addr(3),    tm(TT::Int,   TT::Addr));
            test_binary(op, Int(1),     s("abc"),   tm(TT::Int,   TT::Str));
            test_binary(op, Int(1),     l1.clone(), tm(TT::Int,   TT::List));
            test_binary(op, Int(1),     m1.clone(), tm(TT::Int,   TT::Map));
            test_binary(op, Float(1.0), Bool(true), tm(TT::Float, TT::Bool));
            test_binary(op, Float(1.0), Int(2),     tm(TT::Float, TT::Int));
            test_binary(op, Float(1.0), Addr(2),    tm(TT::Float, TT::Addr));
            test_binary(op, Float(1.0), s("abc"),   tm(TT::Float, TT::Str));
            test_binary(op, Float(1.0), l1.clone(), tm(TT::Float, TT::List));
            test_binary(op, Float(1.0), m1.clone(), tm(TT::Float, TT::Map));
            test_binary(op, s("abc"),   Bool(true), tm(TT::Str,   TT::Bool));
            test_binary(op, s("abc"),   Int(1),     tm(TT::Str,   TT::Int));
            test_binary(op, s("abc"),   Float(2.0), tm(TT::Str,   TT::Float));
            test_binary(op, s("abc"),   l1.clone(), tm(TT::Str,   TT::List));
            test_binary(op, s("abc"),   m1.clone(), tm(TT::Str,   TT::Map));
            test_binary(op, l1.clone(), Bool(true), tm(TT::List,  TT::Bool));
            test_binary(op, l1.clone(), Int(1),     tm(TT::List,  TT::Int));
            test_binary(op, l1.clone(), Float(1.0), tm(TT::List,  TT::Float));
            test_binary(op, l1.clone(), s("abc"),   tm(TT::List,  TT::Str));
            test_binary(op, l1.clone(), l1.clone(), tm(TT::List,  TT::List));
            test_binary(op, l1.clone(), m1.clone(), tm(TT::List,  TT::Map));
            test_binary(op, m1.clone(), Bool(true), tm(TT::Map,   TT::Bool));
            test_binary(op, m1.clone(), Int(1),     tm(TT::Map,   TT::Int));
            test_binary(op, m1.clone(), Float(1.0), tm(TT::Map,   TT::Float));
            test_binary(op, m1.clone(), s("abc"),   tm(TT::Map,   TT::Str));
            test_binary(op, m1.clone(), m1.clone(), tm(TT::Map,   TT::Map));
            test_binary(op, Addr(1),    Bool(true), tm(TT::Addr,  TT::Bool));
            test_binary(op, Addr(1),    Int(2),     tm(TT::Addr,  TT::Int));
            test_binary(op, Addr(1),    s("abc"),   tm(TT::Addr,  TT::Str));
            test_binary(op, Addr(1),    l1.clone(), tm(TT::Addr,  TT::List));
            test_binary(op, Addr(1),    m1.clone(), tm(TT::Addr,  TT::Map));
        }
    }

    #[test]
    fn test_load() {
        assert_evaluates_to(1, 1, Ok(Int(2)), Program {
            code: vec! {LoadI(0)},
            data: vec! {Int(2)}
        });

        assert_evaluates_to(1, 0, Err(Error::IllegalAddr(1)), Program {
            code: vec! {LoadI(1)},
            data: vec! {Int(2)}
        });

        assert_evaluates_to(1, 0, Err(Error::IllegalAddr(0)), Program {
            code: vec! {LoadI(0)},
            data: vec! {}
        });
    }

    #[test]
    fn test_get() {
        let prog = Program {
            code: vec! {LoadI(0), Get},
            data: vec! {s("foo")}
        };

        let env: Env =
            [(String::from("foo"), s("bar"))]
            .iter()
            .cloned()
            .collect();

        assert_eq!(eval(1, 1, prog, env.clone()), Ok(s("bar")));

        let prog = Program {
            code: vec! {LoadI(0), Get},
            data: vec! {s("bar")}
        };

        assert_eq!(
            eval(1, 1, prog, env.clone()),
            Err(Error::KeyError(String::from("bar")))
        );
    }

    #[test]
    fn test_coerce() {
        assert_evaluates_to(1, 1, Ok(Int(0)), Program {
            code: vec! {LoadI(0), Coerce(TT::Int)},
            data: vec! {Value::Bool(false)}
        });

        assert_evaluates_to(1, 1, Ok(Int(1)), Program {
            code: vec! {LoadI(0), Coerce(TT::Int)},
            data: vec! {Value::Bool(true)}
        });

        assert_evaluates_to(1, 1, Ok(Float(1.0)), Program {
            code: vec! {LoadI(0), Coerce(TT::Float)},
            data: vec! {Value::Int(1)}
        });

        assert_evaluates_to(1, 0, tm(TT::Bool, TT::Addr), Program {
            code: vec! {LoadI(0), Coerce(TT::Addr)},
            data: vec! {Value::Bool(true)}
        });

        assert_evaluates_to(1, 0, tm(TT::Int, TT::Addr), Program {
            code: vec! {LoadI(0), Coerce(TT::Addr)},
            data: vec! {Value::Int(0)}
        });

        assert_evaluates_to(1, 0, tm(TT::Float, TT::Addr), Program {
            code: vec! {LoadI(0), Coerce(TT::Addr)},
            data: vec! {Value::Float(0.0)}
        });

        assert_evaluates_to(1, 0, tm(TT::Addr, TT::Addr), Program {
            code: vec! {LoadI(0), Coerce(TT::Addr)},
            data: vec! {Value::Addr(3)}
        });
    }

    #[test]
    fn test_branch() {
        assert_evaluates_to(3, 1, Ok(Int(105)), Program {
            code: vec! {
                LoadI(0),            // 0  [I(100)]
                LoadI(1),            // 1  [I(100) B(T)]
                LoadI(2),            // 2  [I(100) B(T) A(7)]
                BranchTrue,          // 3  [I(100)]
                LoadI(3),            // 4  --
                LoadI(4),            // 5  --
                Branch,              // 6  --
                LoadI(5),            // 7  [I(100) I(5)]
                Binary(Add),         // 8  [I(105)]
            },
            data: vec! {
                Value::Int(100),
                Value::Bool(true),
                Value::Addr(7),
                Value::Int(10),
                Value::Addr(8),
                Value::Int(5)
            }
        });

        assert_evaluates_to(3, 1, Ok(Int(110)), Program {
            code: vec! {
                LoadI(0),            // 0  [I(100)]
                LoadI(1),            // 1  [I(100) B(F)]
                LoadI(2),            // 2  [I(100) B(F) A(7)]
                BranchTrue,          // 3  [I(100)]
                LoadI(3),            // 4  [I(100) I(10)]
                LoadI(4),            // 5  [I(100) I(10) A(8)]
                Branch,              // 6  --
                LoadI(5),            // 7  [I(100) I(10)
                Binary(Add)          // 8  [I(110)]
            },
            data: vec! {
                Value::Int(100),
                Value::Bool(false),
                Value::Addr(7),
                Value::Int(10),
                Value::Addr(8),
                Value::Int(5)
            }
        });

        assert_evaluates_to(3, 1, Ok(Int(105)), Program {
            code: vec! {
                LoadI(0),            // 0  [I(100)]
                LoadI(1),            // 1  [I(100) B(F)]
                LoadI(2),            // 2  [I(100) B(F) A(7)]
                BranchFalse,         // 3  [I(100)]
                LoadI(3),            // 4  [I(100) I(10)]
                LoadI(4),            // 5  [I(100) I(10) A(8)]
                Branch,              // 6  --
                LoadI(5),            // 7  [I(100) I(10)
                Binary(Add)          // 8  [I(110)]
            },
            data: vec! {
                Value::Int(100),
                Value::Bool(false),
                Value::Addr(7),
                Value::Int(10),
                Value::Addr(8),
                Value::Int(5)
            }
        });

        assert_evaluates_to(3, 1, Ok(Int(110)), Program {
            code: vec! {
                LoadI(0),            // 0  [I(100)]
                LoadI(1),            // 1  [I(100) B(F)]
                LoadI(2),            // 2  [I(100) B(F) A(7)]
                BranchFalse,         // 3  [I(100)]
                LoadI(3),            // 4  [I(100) I(10)]
                LoadI(4),            // 5  [I(100) I(10) A(8)]
                Branch,              // 6  --
                LoadI(5),            // 7  [I(100) I(10)
                Binary(Add)          // 8  [I(110)]
            },
            data: vec! {
                Value::Int(100),
                Value::Bool(true),
                Value::Addr(7),
                Value::Int(10),
                Value::Addr(8),
                Value::Int(5)
            }
        });
    }

    #[test]
    fn test_call_ret() {
        // def ftoc(n):
        //     return 5 * (n - 32) / 9
        // ftoc(212)
        assert_evaluates_to(5, 1, Ok(Int(100)), Program {
            code: vec! {
                LoadI(0),           // 0
                Branch,             // 1 goto main
                Arg(0),             // 2 ftoc:
                LoadI(1),           // 3
                Binary(Sub),        // 4
                LoadI(2),           // 5
                Binary(Mul),        // 6
                LoadI(3),           // 7
                Binary(Div),        // 8
                Ret(1),             // 9 return 5 * (n - 32) / 9
                LoadI(4),           // A main:
                LoadI(5),           // B
                Call(1)             // C ftoc(212)
            },
            data: vec! {
                Value::Addr(0x0A),
                Value::Int(32),
                Value::Int(5),
                Value::Int(9),
                Value::Int(212),
                Value::Addr(0x02)
            }
        });

        assert_evaluates_to(5, 1, Ok(Int(100)), Program {
            code: vec! {
                LoadI(0),
                LoadI(1),
                Call(0),
                Halt,
                Ret(0)
            },
            data: vec! {Value::Int(100), Value::Addr(4)}
        });
    }

    #[test]
    fn test_recursion() {
        assert_evaluates_to(25, 1, Ok(Int(120)), Program {
            code: vec! {
                LoadI(0),            // 00
                Branch,              // 01 goto main
                Arg(0),              // 02 fact:
                LoadI(1),            // 03
                Binary(Lte),         // 04
                LoadI(2),            // 05
                BranchFalse,         // 06 if n <= 2
                Arg(0),              // 07
                Ret(1),              // 08 return n
                Arg(0),              // 09 else
                Arg(0),              // 0A
                LoadI(3),            // 0B
                Binary(Sub),         // 0C
                LoadI(4),            // 0D
                Call(1),             // 0E
                Binary(Mul),         // 0F
                Ret(1),              // 10 return n * fact(n - 1)
                LoadI(5),            // 11 main:
                LoadI(4),            // 12
                Call(1)              // 13 fact(5)
            },
            data: vec! {
                Value::Addr(0x11),
                Value::Int(2),
                Value::Addr(0x09),
                Value::Int(1),
                Value::Addr(0x02),
                Value::Int(5)
            }
        });
    }

    #[test]
    fn test_binary_recursion() {
        assert_evaluates_to(25, 1, Ok(Int(34)), Program {
            code: vec! {
                LoadI(0),            // 00
                Branch,              // 01 goto main
                Arg(0),              // 02 fib:
                LoadI(1),            // 03
                Binary(Lte),         // 04
                LoadI(2),            // 05
                BranchFalse,         // 06 if n <= 1
                Arg(0),              // 07
                Ret(1),              // 08 return n
                Arg(0),              // 09 else
                LoadI(3),            // 0A
                Binary(Sub),         // 0B
                LoadI(4),            // 0C
                Call(1),             // 0D  fib(n - 2)
                Arg(0),              // 0E
                LoadI(1),            // 0F
                Binary(Sub),         // 10
                LoadI(4),            // 11
                Call(1),             // 12  fib(n - 1)
                Binary(Add),         // 13
                Ret(1),              // 14 return fib(n - 2) + fib(n - 1)
                LoadI(5),            // 15 main:
                LoadI(4),            // 16
                Call(1)              // 17 fib(9)
            },
            data: vec! {
                Value::Addr(0x15),
                Value::Int(1),
                Value::Addr(0x09),
                Value::Int(2),
                Value::Addr(0x02),
                Value::Int(9)
            }
        });
    }

    #[test]
    fn test_drop() {
        assert_evaluates_to(25, 1, Ok(Int(42)), Program {
            code: vec! {
                LoadI(0),
                LoadI(1),
                LoadI(2),
                Drop(1),
                Binary(Add)
            },
            data: vec! {Value::Int(40), Value::Int(2), Value::Int(100)}
        });
    }

    #[test]
    fn test_dup() {
        assert_evaluates_to(25, 1, Ok(Int(42)), Program {
            code: vec! {LoadI(0), Dup(1), Binary(Add)},
            data: vec! {Value::Int(21)}
        });
    }

    #[test]
    fn test_break() {
        assert_evaluates_to(25, 1, Err(Error::DebugBreak), Program {
            code: vec! {LoadI(0), Break},
            data: vec! {Value::Int(42)}
        });
    }

    #[test]
    fn test_expect() {
        assert_evaluates_to(1, 1, te(!!TT::Bool, TT::Int), Program {
            code: vec! {LoadI(0), Expect(TT::Bool)},
            data: vec! {Value::Int(1)},
        });

        assert_evaluates_to(1, 1, te(!!TT::Int, TT::Bool), Program {
            code: vec! {LoadI(0), Expect(TT::Int)},
            data: vec! {Value::Bool(true)}
        });

        assert_evaluates_to(1, 1, Ok(Float(3.0)), Program {
            code: vec! {LoadI(0), Expect(TT::Float)},
            data: vec! {Value::Float(3.0)}
        });

        assert_evaluates_to(1, 1, Err(Error::Underflow), Program {
            code: vec! {Expect(TT::Addr)},
            data: vec! {}
        });
    }

    #[test]
    fn test_disp() {
        let mut output = Vec::new();
        let prog = Program {
            code: vec! {
                LoadI(0),
                Disp(CairoOp::Rect),
                LoadI(1),
                Disp(CairoOp::Rect),
                LoadI(2),
                Disp(CairoOp::Rect)
            },
            data: vec! {
                Value::Int(1),
                Value::Bool(true),
                Value::Float(1.0)
            }
        };
        let mut vm = VM::new(prog, 1);
        let env = HashMap::new();

        let status = vm.exec(&env, &mut output);
        assert_eq!(status, Ok(()));

        assert_eq!(
            output,
            vec! {Int(1), Bool(true), Float(1.0)}
        );
    }

    #[test]
    fn test_index() {
        assert_evaluates_to(2, 1, Ok(Int(1)), Program {
            code: vec! {
                LoadI(0),
                LoadI(1),
                Index
            },
            data: vec! {l(&[Int(1)]), Value::Addr(0)}
        });

        assert_evaluates_to(2, 1, Err(Error::IndexError(1)), Program {
            code: vec! {
                LoadI(0),
                LoadI(1),
                Index
            },
            data: vec! {l(&[Int(1)]), Value::Addr(1)}
        });

        assert_evaluates_to(2, 1, te(!!TT::Addr, TT::Int), Program {
            code: vec! {
                LoadI(0),
                LoadI(1),
                Index
            },
            data: vec! {l(&[Int(1)]), Value::Int(0)}
        });
    }


    #[test]
    fn test_dot() {
        assert_evaluates_to(2, 1, Ok(Int(1)), Program {
            code: vec! {
                LoadI(0),
                LoadI(1),
                Dot
            },
            data: vec! {m(&[("foo", Int(1))]), s("foo")}
        });

        let key = String::from("bar");
        assert_evaluates_to(2, 1, Err(Error::KeyError(key)), Program {
            code: vec! {
                LoadI(0),
                LoadI(1),
                Dot
            },
            data: vec! {m(&[("foo", Int(1))]), s("bar")}
        });

        assert_evaluates_to(2, 1, te(!!TT::Str, TT::Addr), Program {
            code: vec! {
                LoadI(0),
                LoadI(1),
                Dot
            },
            data: vec! {m(&[("foo", Int(1))]), Value::Addr(0)}
        });
    }
}
