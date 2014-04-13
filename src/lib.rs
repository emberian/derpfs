#![crate_id = "derpfs"]
#![crate_type = "rlib"]
#![crate_type = "dylib"]
#![feature(phase, macro_rules, log_syntax, trace_macros)]

//! The Derp Filesystem

#[phase(syntax, link)] extern crate log;


pub mod disk;
pub mod allocator;
