#![crate_id = "derpfs"]
#![crate_type = "rlib"]
#![crate_type = "bin"]
#![feature(phase, macro_rules, log_syntax, trace_macros)]

//! The Derp Filesystem

#[phase(syntax, link)] extern crate log;

extern crate bitmap;
extern crate hammer;
extern crate uuid;
extern crate libc;

pub use bitmap::Bitmap;

pub mod disk;
pub mod allocator;
pub mod fs;

fn main() {
    let args = std::os::args();
}

fn align(val: uint, align: uint) -> uint {
    val + (val % align)
}

fn block_size(val: uint) -> uint {
    // always have at least one block so we don't need to deal with the mess
    // that is zero-length things
    std::cmp::min(align(val, 4096) / 4096, 1)
}
