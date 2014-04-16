#![crate_id = "derpfs"]
#![crate_type = "bin"]
#![feature(phase, macro_rules, log_syntax, trace_macros)]

//! The Derp Filesystem

#[phase(syntax, link)] extern crate log;

extern crate collections;
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

fn block_size(val: uint) -> u64 {
    // always have at least one block so we don't need to deal with the mess
    // that is zero-length things
    std::cmp::min(align(val, 4096) / 4096, 1) as u64
}

unsafe fn mk_slice(ptr: *mut u8, offset: int, len: uint) -> &mut [u8] {
    let ptr = ptr.offset(offset);
    unsafe { std::cast::transmute( std::raw::Slice { data: ptr as *u8, len: len - offset as uint} ) }
}

macro_rules! opaque (
    ($name:ident) => (
        #[deriving(Show, TotalEq, Eq, Clone, TotalOrd, Ord, Hash)]
        pub struct $name(u64);
        impl $name {
            pub fn new(v: u64) -> $name {
                $name(v)
            }

            pub fn val(&self) -> u64 {
                let $name(v) = *self;
                v
            }
        }
    )
)

opaque!(Id)
opaque!(Offset)
opaque!(StrId)
