
use std::collections::HashMap;

// We construct a program from input text.
pub enum Token {
    Id(&str),
    Str(&str),
    Int(&str),
    Float(&str),
    Op(&str)
}


// TBD
