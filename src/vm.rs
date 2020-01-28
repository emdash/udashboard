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


// Arithmetic and logic operations
#[derive(Copy,Clone)]
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
#[derive(Copy,Clone)]
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
#[derive(Copy,Clone)]
enum Opcode {
    Push(Immediate),
    Load,
    Coerce(TypeTag),
    Binary(BinOp),
    Unary(UnOp),
    Label,
    Call,
    Ret,
    BranchTrue,
    BranchFalse,
    Jump,
    Repeat,
    Drop(u8),
    Dup(u8),    // If you need more than 255 copies, something is wrong.
    Index,
    Dot,
    Get,
    Swap,
    Rel(u8),
    Expect,
    Disp,
    Break
}


// The result of any operation
type Result<T> = core::result::Result<T, Error>;


// All valid values
//
// TODO: some sensible strategy for handling strings.
// Todo: add the container types.
#[derive(Copy, Clone, Debug)]
pub enum Value {
    Bool(bool),
    //    Str(Rc<String>),
    Int(i64),
    Float(f64),
    // TODO: different pointer types for different locations.
    Addr(usize)
}


// It kinda bugs me that I need this, but Rust doesn't have a way of
// exposing an enum's discriminant besides a pattern match.
#[derive(Copy, Clone, Debug)]
pub enum TypeTag {
    Bool,
    Int,
    Float,
    Addr
}


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
fn expected(expected: TypeTag, got: Value) -> Error {
    Error::TypeError { expect: expected, got: got.into() }
}

// Construct an Error::TypeMismatch from a value.
fn type_mismatch(a: Value, b: Value) -> Error {
    Error::TypeMismatch(a.into(), b.into())
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
    (un $name:ident { $( $p:pat => $e:expr ),+ } ) => {
        pub fn $name (self) -> Result<Value> {
            // Bringing Value into scope saves us some characters
            use Value::*;
            match self {
                $($p => Ok($e)),+ ,
                value => Err(expected(TypeTag::Int, value))
            }
        }
    };

    // Template ofr a binary operator
    (bin $name:ident { $( $p:pat => $e:expr ),+ } ) => {
        pub fn $name (self, other: Value) -> Result<Value> {
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
            (a,               b)
                => Err(Error::TypeMismatch(a.into(), b))
        }
    }

    operator! { un abs {
        Int(value)   => Value::Int(value.abs()),
        Float(value) => Value::Float(value.abs())
    } }

    operator! { bin pow {
        // XXX: silent coercion to u32.
        (Int(a),   Int(b))   => Value::Int(a.pow(b as u32)),
        (Float(a), Float(b)) => Value::Float(a.powf(b))
    } }

    operator! { bin min {
        // XXX: silent coercion to u32.
        (Int(a),   Int(b))   => Value::Int(a.min(b)),
        (Float(a), Float(b)) => Value::Float(a.min(b))
    } }

    operator! { bin max {
        // XXX: silent coercion to u32.
        (Int(a),   Int(b))   => Value::Int(a.max(b)),
        (Float(a), Float(b)) => Value::Float(a.max(b))
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

    operator! { bin shl { (Int(a), Int(b)) => Int(a >> b) } }
    operator! { bin shr { (Int(a), Int(b)) => Int(a << b) } }
    operator! { un  not { Bool(a) => Bool(!a) } }
    operator! { un  neg { Int(a) => Int(-a) } }

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
        (Int(a), Int(b)) => Bool(a == b),
        (Float(a), Float(b)) => Bool(a == b)
    } }
}


// Factor out boilerplate for implementation of TryInto 
macro_rules! impl_try_into {
    ($variant:ident => $type:ident) => {
        impl TryInto<$type> for Value {
            fn try_into(self) -> Result<$type> {
                match self {
                    Value::$variant(value) => Ok(value),
                    v => Err(expected(TypeTag::$variant, v))
                }
            }
        }
    }
}

impl_try_into! { Bool  => bool }
impl_try_into! { Int   => i64 }
impl_try_into! { Float => f64 }
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

// We can always obtain a TypeTag for a value.
impl Into<TypeTag> for Value {
    fn into(self) -> TypeTag {
        match self {
            Value::Bool(_)  => TypeTag::Bool,
            Value::Int(_)   => TypeTag::Int,
            Value::Float(_) => TypeTag::Float,
            Value::Addr(_)  => TypeTag::Addr
        }
    }
}


/******************************************************************************/


#[derive(Copy, Clone, Debug)]
// This type holds constant values as well as storage for strings,
// constant lists, and objects.
pub enum ConstValue {
    Val(Value), /*
    Str(Box<String>),
    List(Box<Vec<Value>>),
    Map(Box<HashMap<String, Value>>) */
}


/******************************************************************************/

// This is another crucial value type, especially because it's
// propagated up the stack.
#[derive(Copy, Clone, Debug)]
pub enum Error {
    Underflow,
    Overflow,
    NotImplemented,
    IllegalOpcode,
    IllegalAddr(usize),
    TypeError {
        expect: TypeTag,
        got: TypeTag
    },
    TypeMismatch(TypeTag, TypeTag),
    //NameError(Rc<String>),
    IndexError(usize),
    DebugBreak,
    Halt,
}


type Stack = Vec<Value>;
type Env = HashMap<String, ConstValue>;


// The internal program representation.
//
// To avoid embedding string data into the Opcode enum, we instead
// store an index into a global table of string data.
//
// Code is a sequence of instructions.
// Data is the table of string values.
pub struct Program {
    code: Vec<Opcode>,
    data: Vec<ConstValue>
}


impl Program {
    // Safely fetch the opcode from the given address.
    //
    // The address is simply the index into the instruction sequence.
    fn fetch(&self, index: usize) -> Result<Opcode> {
        if index < self.code.len() {
            Ok(self.code[index])
        } else {
            Err(Error::IllegalAddr(index))
        }
    }
/*
    // Safely retrieve the global static data from the given address.
    //
    // The address is simply the index into the data section.
    pub fn load(&self, index: usize) -> Result<Value> {
        if index < self.data.len() {
            match self.data[index] {
                Constval::Val(v) => Ok(v),
                _      => Err(Error::NotImplemented)
            }
        } else {
            Err(Error::IllegalAddr(index))
        }
    }
*/
}


// The entire VM state.
pub struct VM {
    program: Program,
    stack: Stack,
    pc: usize,
}


// The type of control flow an instruction can have.
pub enum ControlFlow {
    Advance,
    Branch(usize),
    Yield(Value),
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

    // Run the entire program until it halts.
    pub fn exec(&mut self, env: &Env) -> Result<()> {
        // Safe, because we have borrowed env and so by contract it
        // is immutable.
        loop { unsafe { self.step(env)? } }
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
    pub unsafe fn step(&mut self, env: &Env) -> Result<()> {
        let opcode = self.program.fetch(self.pc)?;
        let result = self.dispatch(opcode, env)?;

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
        Err(Error::NotImplemented)
    }


    // Dispatch opcode to the Value implementation.
    fn binop(&mut self, op: BinOp) -> Result<ControlFlow> {
        let b = self.pop()?;
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
            BinOp::Shr  => a.shl(b),
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

    // Push current PC onto stack, and jump.
    fn call(&mut self) -> Result<ControlFlow> {
        let target: usize = self.pop_into()?;
        self.push(Value::Addr(self.pc))?;
        Ok(ControlFlow::Branch(target))
    }

    // Return from subroutine.
    fn ret(&mut self) -> Result<ControlFlow> {
        let target: usize = self.pop_into()?;
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
    fn jump(&mut self) -> Result<ControlFlow> {
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
        for _ in 0..n { self.push(top)?; }
        Ok(ControlFlow::Advance)
    }

    // Swap the top stack values
    fn swap(&mut self) -> Result<ControlFlow> {
        let b = self.pop()?;
        let a = self.pop()?;
        self.push(b)?;
        self.push(a)
    }

    // Emit the top of stack as output.
    fn disp(&mut self) -> Result<ControlFlow> {
        let value = self.pop()?;
        self.emit(value);
        Ok(ControlFlow::Advance)
    }

    // Provided by trait implementatation
    fn emit(&self, value: Value) {
        // This will be generalized later
        println!("{:?}", value);
    }

    // Dispatch table for built-in opcodes
    fn dispatch(&mut self, op: Opcode, _: &Env) -> Result<ControlFlow> {
        match op {
            Opcode::Push(i)     => self.push(i.into()),
            Opcode::Load        => self.load(),
            Opcode::Coerce(t)   => self.coerce(t),
            Opcode::Binary(op)  => self.binop(op),
            Opcode::Unary(op)   => self.unop(op),
            Opcode::Label       => self.push(Value::Addr(self.pc)),
            Opcode::Call        => self.call(),
            Opcode::Ret         => self.ret(),
            Opcode::BranchTrue  => self.branch_true(),
            Opcode::BranchFalse => self.branch_false(),
            Opcode::Jump        => self.jump(),
            Opcode::Drop(n)     => self.drop(n),
            Opcode::Dup(n)      => self.dup(n),
            Opcode::Swap        => self.swap(),
            Opcode::Disp        => self.disp(),
            Opcode::Break       => Err(Error::DebugBreak),
            _                   => Err(Error::IllegalOpcode)
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use Opcode::*;
    use BinOp::*;
    use UnOp::*;
    use Value::*;

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
        vm.exec(&env);

        let result: i64 = vm.pop().unwrap().try_into().unwrap();
        assert_eq!(result, 3);
    }
}

