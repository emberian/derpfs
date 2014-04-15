#![crate_id = "derpfs"]
#![crate_type = "rlib"]
#![crate_type = "bin"]
#![feature(phase, macro_rules, log_syntax, trace_macros)]

//! The Derp Filesystem

#[phase(syntax, link)] extern crate log;

extern crate bitmap;
extern crate hammer;
extern crate uuid;
extern crate fuse;
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

unsafe fn mk_slice(ptr: *mut u8, offset: int, len: uint) -> &mut [u8] {
    let ptr = ptr.offset(offset);
    unsafe { std::cast::transmute( std::raw::Slice { data: self.map.data as *u8, len: len - offset} ) };
}

macro_rules! opaque (
    ($name:ident) => (
        pub struct $ident(u64);
        impl $ident {
            pub fn new(v: u64) -> $ident {
                $ident(v)
            }

            pub fn val(&self) -> u64 {
                let $ident(v) = *self;
                v
            }
        }
    )
)

opaque!(Id)
opaque!(Offset)
opaque!(StrId)
