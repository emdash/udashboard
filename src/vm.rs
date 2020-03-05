// (C) 2020 Brandon Lewis
//
// A virtual machine for a custom Elm-inspired graphics system.
//
// This system is optimized for short-running programs that get
// executed repeatedly on data that changes at "interactive"
// frequencies of 10-60hz. In other words, it's a language for
// animation, UI, and real-time display. It's assumed that both the
// code, and the data being processed, could be malicious. And it's
// assumed that it's running on low-end hardware, so a great deal of
// attention has been paid to space efficiency, low overhead, and
// memory safety.
//
// The capabilities of the instruction set are carefully designed to
// place restrictions on the runtime behavior of the system in order
// to guarantee determinism and place a reasonable upper bound on
// memory requirements. As an added bonus, it should be cache
// efficient. In particular, all mutable state must live on a stack
// with a fixed upper size limit; and, while side effects are allowed,
// they can only have an external influence, and do not affect the
// behavior of subsequent instructions -- provided that embedding
// applications respect certain rules discussed below.
//
// Conceptually, a program is a sequence of stack operations evaluated
// within a read-only "environment", producing sequence of opaque
// "effects" as output. For a given progaram and environment, the
// sequence of effects is completely deterministic, though not
// necessarily valid.
//
// There are different ways to frame this conceptually:
// - as a compression scheme, which can be expanded to yield a
//   sequence of values.
// - as a signal processing system, which transforms arbitray inputs
//   to arbitray outputs in constant space and deterministic time.
// - as a runtime for either a functional *or* proceedural language.
// - as a family of DFAs.
//
// *Validity*
//
// The set of runtime errors is represented by the Error enum in this
// file. All are non-recoverable, modulo an external debugger.
//
// For our purposes here, a valid program is one which terminates
// without an error, for a given stack limit and environment. A
// well-behaved program is one that can be proven valid for a given
// stack limit and environment "shape".
//
// I believe it will be possible to quantify space requirements for a
// given program automatically, through an efficient analysis of the
// instruction sequence. It may also be possible quantify time
// requirements in a similar way. This would put reasonable limits on
// the kind of chaos that might otherwise ensue from executing
// arbitrary code from untrusted sources.
//
// *Safety*
//
// Safety is naturally implementation-dependent. Much of the
// guarantees here depend on run-time error detection which, in the
// age of spectre, is not reliable. Moreover, it is up to the
// embedding code not to provide the VM with fundamentally unsafe
// capabilities via the Effects mechanism.
//
// The goal here is simply to not exacerbate the problem with an
// instruction set that is fundamentally inscrutable, and full of
// implementation quirks, AKA "weird machines" that have proven to be
// fertile ground for vulnerabilities to fester.
//
// Crucially, the embedding application has the responsibility to
// uphold the following contract: *it must not allow effects to write
// back to the environment during program execution*!  This is
// crucial!  The API is designed to prevent this from happening by
// accident, but I can't stop you form using "unsafe", "RefCell" or
// other hacks to defeat this intentional restriction. You have been
// warned.
//
// *Instructions*
//
// The core instruction set is broadly similar to other
// stack-machines. The usual family of arithmetic, logic, and stack
// manipulation operators are providd.
//
// Subroutines are supported with the "call", "ret", and "rel"
// instructions. "call" and "ret" handle the return address and stack
// frame, while "rel" allows stable indexing of function
// parameters. This greatly simplifies compilation of high-level code,
// while keeping *all* operands on the stack.
//
// Control flow is provided by "bt", "bf", and "jmp" instructions,
// which take *addresses* rather than abstract "blocks" as you see in
// fourth. This is for the sake of efficiency. I don't like the
// overhead that blocks require, seems wasted with an in-memory
// representation. This is a low level ISA intended to serve as a
// target for a higher-level language. I could be convinced
// otherwise. But the way I see it, you have to do some form of linear
// scan over the bytecode at load time, and it's easy enough to
// calculate addresses during that phase.
//
// *Values*
//
// - int, float, char, string, id, list, map, effect, addr (see below).
//
// Arithmetic is allowed only on int and float types. There is no
// silent coercion.
//
// String, list, and map types are immutable, and static for the
// duration of the VM program.
//
// String subsetting is supported, but not string construction. If you
// want to concatenate or format strings, you must do it externally,
// via the effect mechanism. If you want to display values for debug,
// pack them into an Effect and `disp` the effect.
//
// List and map types suport "get" and "iter" instructions. For list
// types, "iter" is guaranteed to traverse in order. For map types,
// it's not. Calling "get" on a list requires an integer key, while
// calling "get" on a map requries a string key.
//
// Addrs are an unsigned index type, used with fetch
// instructions. These are *logical* indices, not raw pointers. While
// the Addr is used in a variety of contexts, it's important to
// understand that by design there are separate address spaces for
// instructions, static data, the environment, and the stack. In
// addition, no opcodes support calculations on addresses at
// runtime. I may relax this restriction slightly, for the sake of
// being able to support jump tables. If I do, it will take the form
// of an 8-bit immediate operand on the Jmp instruction. I need to be
// convinced it's relatively safe to allow this.
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
// An Effects is an extensible symbolic representation of the external
// behavior the VM is controlling. You can think of values as
// "flowing" through the program from a "source" to a "drain". A
// source may be the environment, value from static data, or an
// immediate operand. A "drain" is either "drop", or "disp"
// instruction.
//
// While "drop" and "disp" work on arbitrary values, but "drop"
// behaves specially when the value is an "effect object". You
// construct an effect on the stack with the "effect" instruction,
// which takes an opaque "tag" broadly classifying the effect. You can
// then append VM values into it with the "pack" instruction.
//
// "pack" takes an effect, and a single VM value. It moves the value
// from the stack into the effect, leaving the effect on stack. This
// allows you to incrementally build arbitray structure
// incrementally. Finally, you "drop" the effect, and it will be
// handed off to the application code into which the VM is embedded.
//
// This means that, like most opcodes, pack and drop can fail. This
// can happen if the effect is rejected by the client, you attempt to
// pack an illegal value into an effect, or if you exceed the an
// arbirary maximum effect limit. All of these things are under the
// control of the embedding application, *not* the VM program.
//
// *Summary*
//
// - Designed to be repeatedly executed at interactive frequencies.
// - Runs in constant space, with a stack limit set by user at runtime.
// - Stack-based, postfix order.
// - Designed for safety and speed.
// - The eventual goal is to be panic-free. Will need tooling to
//   verify this.


use std::collections::HashMap;
use std::io::Stdout;
use std::rc::Rc;
use enumflags2::BitFlags;


// Arithmetic and logic operations
#[derive(Copy, Clone, Debug)]
enum BinOp {
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
    Eq,
    Shl,
    Shr,
    Min,
    Max
}


#[derive(Copy, Clone, Debug)]
enum UnOp {
    Not,
    Neg,
    Abs,
}


// Immediate values used by push instruction.
//
// TODO: tune these for size.  The width of this type essentially
// deterines the width of the entire opcode. There's a trade-off
// between having to do extra work to load large constants, and
// blowing out the intruction cache with larger prorams. I just don't
// know what the right answer is.
//
// We need to be able to load floating point values, and silently
// truncating floating point literals to 32-bit is bad, since it can
// change the value. There are ways around this, but let's not worry
// about them just yet.
//
// For future reference, ideas include:
// - opcodes to alias int / float
// - some pair of li / lui opcodes that get the job done with reasonable
//   overhead.
// - opcodes for directly setting exponent and mantissa, which could be
//   useful in their own right.
// - use the data section for floating-point immmediates, which isn't
//   as bad a waste of space as, say, an 8-bit float.
#[derive(Copy, Clone, Debug)]
enum Immediate {
    Bool(bool),
    Int(i16),    // no issues here, integers are exact.
    Float(f64),  // this is the culprit here, because of rounding.
    Addr(usize)  // and this, to a lesser degree.
}


// The in-memory opcode format.
//
// This is designed to make illegal operations impossible to
// represent, thereby avoiding "wierd machines" resulting from
// ill-formed opcodes.
//
// The downside is that the actual representation may be very large,
// especially considering struct padding and alignment. The exact
// layout is is up to the rust compiler. On the one hand, opcode
// access itself should be reasonably efficient. On the other hand,
// the program may not fit well into cache.
//
// It's not yet clear how much any of this will matter, because in
// theory at least, the VM execution overhead is vastly dominated by
// the cairo rendering pass. Won't know until I get it working and can
// do some benchmarking.
//
// For now, the plan is just to get it working. Even if it takes many
// bytes to represent an instruction, it could still be vastly more
// compact than the equivalent text, python, or javascript
// representation. Decode and dispatch could also be much faster,
// given that most of this should compile down to jump tables.
//
// I would be *very* curious to look at the disassembly.
//
// The good news is that doing it this way gives maximum flexibility
// for future optimization. For now, just getting it working is top
// priority.
#[derive(Copy, Clone, Debug)]
enum Opcode {
    Push(Immediate),
    Load,
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
    Get,
    Expect(TypeTag),
    Disp,
    Break,
}


// The result of any operation
type Result<T> = core::result::Result<T, Error>;


// All valid values
//
// TODO: some sensible strategy for handling strings.
// Todo: add the container types.
#[derive(Clone, Debug)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<String>),
    List(Rc<Vec<Value>>),
    Map(Rc<HashMap<String, Value>>),
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
trait TryInto<T> {
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
    operator! { un not (BitFlags::from_flag(TypeTag::Bool)) { Bool(a) => Bool(!a) } }
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
        _                    => Bool(false)
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
impl_try_into! { Map   => Rc<HashMap<String, Value>> }
impl_try_into! { Addr  => usize }


// This probably could just be an associated function, rather than a
// trait.
impl From<Immediate> for Value {
    fn from(v: Immediate) -> Value {
        match v {
            Immediate::Bool(v)  => Value::Bool(v as bool),
            Immediate::Int(v)   => Value::Int(v as i64),
            Immediate::Float(v) => Value::Float(v as f64),
            Immediate::Addr(v)  => Value::Addr(v as usize)
        }
    }
}


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
type Env = HashMap<String, Value>;


// The internal program representation.
//
// To avoid embedding string data into the Opcode enum, we instead
// store an index into a global table of string data.
//
// Code is a sequence of instructions.
// Data is the table of string values.
pub struct Program {
    code: Vec<Opcode>,
    data: Vec<Value>
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
    fn output(&mut self, value: Value);
}


impl Output for Stdout {
    fn output(&mut self, value: Value) {
        println!("{:?}", value);
    }
}

impl Output for Vec<Value> {
    fn output(&mut self, value: Value) {
        self.push(value)
    }
}

impl Output for () {
    fn output(&mut self, _value: Value) {
    }
}


// Somewhat naive implementation. Not optimal, but hopefully safe.
//
// TODO: Store borrow of Env internally, so we an make `step` safe,
// and then implement `Iterator`.
//
// TODO: Implement in-place stack mutation, and benchmark to see if it
// offers any improvement.
//
// TODO: Implement remaining opcodes.
//
// TODO: Effects.
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
    pub fn exec(&mut self, env: &Env, out: &mut impl Output) -> Result<()> {
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
    pub unsafe fn step(&mut self, env: &Env, out: &mut impl Output) -> Result<()> {
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
            Ok(Value::Addr(address)) => {
                self.push(self.program.load(address)?)?;
                Ok(ControlFlow::Advance)
            },
            Ok(v) => Err(expected(BitFlags::from_flag(TypeTag::Addr), &v)),
            Err(e) => Err(e)
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
    fn pop_into<T>(&mut self) -> Result<T> where Value: TryInto<T> {
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
                self.push(self.stack[index].clone())?;
                Ok(ControlFlow::Advance)
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
            self.push(list[index].clone());
            Ok(ControlFlow::Advance)
        } else {
            Err(Error::IndexError(index))
        }
    }

    // Return element from a map reference
    fn dot(&mut self) -> Result<ControlFlow> {
        let key: Rc<String> = self.pop_into()?;
        let key = key.to_string();
        let map: Rc<HashMap<String, Value>> = self.pop_into()?;
        if let Some(value) = map.get(&key) {
            self.push(value.clone());
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
    fn disp(&mut self, out: &mut Output) -> Result<ControlFlow> {
        let value = self.pop()?;
        out.output(value);
        Ok(ControlFlow::Advance)
    }

    // Provided by trait implementatation
    fn emit(&self, value: Value) {
        // This will be generalized later
        println!("{:?}", value);
    }

    // Dispatch table for built-in opcodes
    fn dispatch(&mut self, op: Opcode, _: &Env, out: &mut Output) -> Result<ControlFlow> {
        match op {
            Opcode::Push(i)     => self.push(i.into()),
            Opcode::Load        => self.load(),
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
            Opcode::Disp        => self.disp(out),
            Opcode::Break       => Err(Error::DebugBreak),
            _                   => Err(Error::IllegalOpcode)
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
    use Opcode::*;
    use BinOp::*;
    use UnOp::*;
    use Value::*;
    use Immediate as I;
    use TypeTag as TT;

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
        prog: Program
    ) -> Result<Value> {
        let mut vm = VM::new(prog, stack_limit);
        let env = HashMap::new();
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
        let result = eval(stack_limit, expected_final_depth, prog);
        println!("assert_evaluates_to: {:?} == {:?})", &expected_value, &result);
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
        println!("test_unary({:?})", op);
        assert_evaluates_to(1, single_op_depth(&expected), expected, Program {
            code: vec! {
                Push(I::Addr(0)),
                Load,
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
        println!("test_binary({:?})", op);
        assert_evaluates_to(2, single_op_depth(&expected), expected, Program {
            code: vec! {
                Push(I::Addr(0)),
                Load,
                Push(I::Addr(1)),
                Load,
                Binary(op)
            },
            data: vec! {a, b}
        });
    }

    #[test]
    fn test_simple() {
        let p = Program {
            code: vec! {
                Push(Immediate::Int(1)),
                Push(Immediate::Int(2)),
                Binary(Add)
            },
            data: vec! {}
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

        test_unary(Not, Int(1), te(!!TT::Bool, TT::Int));
        test_unary(Neg, Int(1), Ok(Int(-1)));
        test_unary(Abs, Int(-1), Ok(Int(1)));

        test_unary(Not, Float(1.0), te(!!TT::Bool, TT::Float));
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
            code: vec! {
                Push(I::Addr(0)),
                Load
            },
            data: vec! { Int(2) }
        });

        assert_evaluates_to(1, 0, Err(Error::IllegalAddr(1)), Program {
            code: vec! {
                Push(I::Addr(1)),
                Load
            },
            data: vec! { Int(2) }
        });

        assert_evaluates_to(1, 0, Err(Error::IllegalAddr(0)), Program {
            code: vec! {
                Push(I::Addr(0)),
                Load
            },
            data: vec! {}
        });
    }

    #[test]
    fn test_coerce() {
        assert_evaluates_to(1, 1, Ok(Int(0)), Program {
            code: vec! {
                Push(I::Bool(false)),
                Coerce(TypeTag::Int)
            },
            data: vec! {}
        });
        assert_evaluates_to(1, 1, Ok(Int(1)), Program {
            code: vec! {
                Push(I::Bool(true)),
                Coerce(TypeTag::Int)
            },
            data: vec! {}
        });

        assert_evaluates_to(1, 1, Ok(Float(1.0)), Program {
            code: vec! {
                Push(I::Int(1)),
                Coerce(TypeTag::Float)
            },
            data: vec! {}
        });

        assert_evaluates_to(1, 0, tm(TT::Bool, TT::Addr), Program {
            code: vec! {
                Push(I::Bool(true)),
                Coerce(TypeTag::Addr)
            },
            data: vec! {}
        });

        assert_evaluates_to(1, 0, tm(TT::Int, TT::Addr), Program {
            code: vec! {
                Push(I::Int(0)),
                Coerce(TypeTag::Addr)
            },
            data: vec! {}
        });

        assert_evaluates_to(1, 0, tm(TT::Float, TT::Addr), Program {
            code: vec! {
                Push(I::Float(0.0)),
                Coerce(TypeTag::Addr)
            },
            data: vec! {}
        });

        assert_evaluates_to(1, 0, tm(TT::Addr, TT::Addr), Program {
            code: vec! {
                Push(I::Addr(3)),
                Coerce(TypeTag::Addr)
            },
            data: vec! {}
        });
    }

    #[test]
    fn test_branch() {
        assert_evaluates_to(3, 1, Ok(Int(105)), Program {
            code: vec! {
                Push(I::Int(100)),   // 0  [I(100)]
                Push(I::Bool(true)), // 1  [I(100) B(T)]
                Push(I::Addr(7)),    // 2  [I(100) B(T) A(7)]
                BranchTrue,          // 3  [I(100)]
                Push(I::Int(10)),    // 4  --
                Push(I::Addr(8)),    // 5  --
                Branch,              // 6  --
                Push(I::Int(5)),     // 7  [I(100) I(5)]
                Binary(Add),         // 8  [I(105)]
            },
            data: vec! {}
        });

        assert_evaluates_to(3, 1, Ok(Int(110)), Program {
            code: vec! {
                Push(I::Int(100)),    // 0  [I(100)]
                Push(I::Bool(false)), // 1  [I(100) B(F)]
                Push(I::Addr(7)),     // 2  [I(100) B(F) A(7)]
                BranchTrue,           // 3  [I(100)]
                Push(I::Int(10)),     // 4  [I(100) I(10)]
                Push(I::Addr(8)),     // 5  [I(100) I(10) A(10)]
                Branch,               // 6  [I(100) I(10)
                Push(I::Int(5)),      // 7  ---
                Binary(Add),          // 8  [I(110)]
            },
            data: vec! {}
        });

        assert_evaluates_to(3, 1, Ok(Int(105)), Program {
            code: vec! {
                Push(I::Int(100)),    // 0  [I(100)]
                Push(I::Bool(false)), // 1  [I(100) B(T)]
                Push(I::Addr(7)),     // 2  [I(100) B(T) A(7)]
                BranchFalse,          // 3  [I(100)]
                Push(I::Int(10)),     // 4  --
                Push(I::Addr(8)),     // 5  --
                Branch,               // 6  --
                Push(I::Int(5)),      // 7  [I(100) I(5)]
                Binary(Add),          // 8  [I(105)]
            },
            data: vec! {}
        });

        assert_evaluates_to(3, 1, Ok(Int(110)), Program {
            code: vec! {
                Push(I::Int(100)),    // 0  [I(100)]
                Push(I::Bool(true)),  // 1  [I(100) B(F)]
                Push(I::Addr(7)),     // 2  [I(100) B(F) A(7)]
                BranchFalse,          // 3  [I(100)]
                Push(I::Int(10)),     // 4  [I(100) I(10)]
                Push(I::Addr(8)),     // 5  [I(100) I(10) A(10)]
                Branch,               // 6  [I(100) I(10)
                Push(I::Int(5)),      // 7  ---
                Binary(Add),          // 8  [I(110)]
            },
            data: vec! {}
        });
    }

    #[test]
    fn test_call_ret() {
        // def ftoc(n):
        //     return 5 * (n - 32) / 9
        // ftoc(212)
        assert_evaluates_to(5, 1, Ok(Int(100)), Program {
            code: vec! {
                Push(I::Addr(0xA)), // 0
                Branch,             // 1 goto main
                Arg(0),             // 2 ftoc:
                Push(I::Int(32)),   // 3
                Binary(Sub),        // 4
                Push(I::Int(5)),    // 5
                Binary(Mul),        // 6
                Push(I::Int(9)),    // 7
                Binary(Div),        // 8
                Ret(1),             // 9 return 5 * (n - 32) / 9
                Push(I::Int(212)),  // A main:
                Push(I::Addr(0x2)), // B
                Call(1)             // C ftoc(212)
            },
            data: vec! {}
        });
    }

    #[test]
    fn test_recursion() {
        assert_evaluates_to(25, 1, Ok(Int(120)), Program {
            code: vec! {
                Push(I::Addr(0x11)), // 00
                Branch,              // 01 goto main
                Arg(0),              // 02 fact:
                Push(I::Int(2)),     // 03
                Binary(Lte),         // 04
                Push(I::Addr(0x09)), // 05
                BranchFalse,         // 06 if n <= 2
                Arg(0),              // 07
                Ret(1),              // 08 return n
                Arg(0),              // 09 else
                Arg(0),              // 0A
                Push(I::Int(1)),     // 0B
                Binary(Sub),         // 0C
                Push(I::Addr(0x02)), // 0D
                Call(1),             // 0E
                Binary(Mul),         // 0F
                Ret(1),              // 10 return n * fact(n - 1)
                Push(I::Int(5)),     // 11 main:
                Push(I::Addr(0x02)), // 12
                Call(1)              // 13 fact(5)
            },
            data: vec! {}
        });
    }

    #[test]
    fn test_binary_recursion() {
        assert_evaluates_to(25, 1, Ok(Int(34)), Program {
            code: vec! {
                Push(I::Addr(0x15)), // 00
                Branch,              // 01 goto main
                Arg(0),              // 02 fib:
                Push(I::Int(1)),     // 03
                Binary(Lte),         // 04
                Push(I::Addr(0x09)), // 05
                BranchFalse,         // 06 if n <= 1
                Arg(0),              // 07
                Ret(1),              // 08 return n
                Arg(0),              // 09 else
                Push(I::Int(2)),     // 0A
                Binary(Sub),         // 0B
                Push(I::Addr(0x02)), // 0C
                Call(1),             // 0D  fib(n - 2)
                Arg(0),              // 0E
                Push(I::Int(1)),     // 0F
                Binary(Sub),         // 10
                Push(I::Addr(0x02)), // 11
                Call(1),             // 12  fib(n - 1)
                Binary(Add),         // 13
                Ret(1),              // 14 return fib(n - 2) + fib(n - 1)
                Push(I::Int(9)),     // 15 main:
                Push(I::Addr(0x02)), // 16
                Call(1)              // 17 fib(9)
            },
            data: vec! {}
        });
    }

    #[test]
    fn test_drop() {
        assert_evaluates_to(25, 1, Ok(Int(42)), Program {
            code: vec! {
                Push(I::Int(40)),
                Push(I::Int(2)),
                Push(I::Int(100)),
                Drop(1),
                Binary(Add)
            },
            data: vec! {}
        });
    }

    #[test]
    fn test_dup() {
        assert_evaluates_to(25, 1, Ok(Int(42)), Program {
            code: vec! {
                Push(I::Int(21)),
                Dup(1),
                Binary(Add)
            },
            data: vec! {}
        });
    }

    #[test]
    fn test_break() {
        assert_evaluates_to(25, 1, Err(Error::DebugBreak), Program {
            code: vec! {
                Push(I::Int(42)),
                Break
            },
            data: vec! {}
        });
    }

    #[test]
    fn test_expect() {
        assert_evaluates_to(1, 1, te(!!TT::Bool, TT::Int), Program {
            code: vec! {
                Push(I::Int(1)),
                Expect(TT::Bool)
            },
            data: vec! {}
        });

        assert_evaluates_to(1, 1, te(!!TT::Int, TT::Bool), Program {
            code: vec! {
                Push(I::Bool(true)),
                Expect(TT::Int)
            },
            data: vec! {}
        });

        assert_evaluates_to(1, 1, Ok(Float(3.0)), Program {
            code: vec! {
                Push(I::Float(3.0)),
                Expect(TT::Float)
            },
            data: vec! {}
        });

        assert_evaluates_to(1, 1, Err(Error::Underflow), Program {
            code: vec! {
                Expect(TT::Addr)
            },
            data: vec! {}
        });
    }

    #[test]
    fn test_disp() {
        let mut output = Vec::new();
        let prog = Program {
            code: vec! {
                Push(I::Int(1)),
                Disp,
                Push(I::Bool(true)),
                Disp,
                Push(I::Float(1.0)),
                Disp
            },
            data: vec! {}
        };
        let mut vm = VM::new(prog, 1);
        let env = HashMap::new();
        let status = vm.exec(&env, &mut output);

        assert_eq!(
            output,
            vec! {Value::Int(1), Value::Bool(true), Value::Float(1.0)}
        );
    }

    #[test]
    fn test_index() {
        assert_evaluates_to(2, 1, Ok(Int(1)), Program {
            code: vec! {
                Push(I::Addr(0)),
                Load,
                Push(I::Addr(0)),
                Index
            },
            data: vec! {l(&[Int(1)])}
        });

        assert_evaluates_to(2, 1, Err(Error::IndexError(1)), Program {
            code: vec! {
                Push(I::Addr(0)),
                Load,
                Push(I::Addr(1)),
                Index
            },
            data: vec! {l(&[Int(1)])}
        });

        assert_evaluates_to(2, 1, te(!!TT::Addr, TT::Int), Program {
            code: vec! {
                Push(I::Addr(0)),
                Load,
                Push(I::Int(1)),
                Index
            },
            data: vec! {l(&[Int(1)])}
        });
    }


    #[test]
    fn test_dot() {
        assert_evaluates_to(2, 1, Ok(Int(1)), Program {
            code: vec! {
                Push(I::Addr(0)),
                Load,
                Push(I::Addr(1)),
                Load,
                Dot
            },
            data: vec! {m(&[("foo", Int(1))]), s("foo")}
        });

        let key = String::from("bar");
        assert_evaluates_to(2, 1, Err(Error::KeyError(key)), Program {
            code: vec! {
                Push(I::Addr(0)),
                Load,
                Push(I::Addr(1)),
                Load,
                Dot
            },
            data: vec! {m(&[("foo", Int(1))]), s("bar")}
        });

        assert_evaluates_to(2, 1, te(!!TT::Str, TT::Addr), Program {
            code: vec! {
                Push(I::Addr(0)),
                Load,
                Push(I::Addr(1)),
                Dot
            },
            data: vec! {m(&[("foo", Int(1))])}
        });
    }
}
