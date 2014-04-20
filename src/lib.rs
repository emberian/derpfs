//! The Derp Filesystem

#![crate_id = "derpfs"]
#![crate_type = "bin"]
#![feature(macro_rules)]
#![feature(globs)]

extern crate collections;
extern crate getopts;
extern crate native;
extern crate rand;
extern crate libc;
extern crate uuid;

extern crate bitmap;
extern crate fuse;

use getopts::{usage, optopt, optflag, getopts};
use std::sync::atomics;

pub mod fs;
pub mod disk;

// XXX: basically every use of uint or int is incorrect. Due to the heavy
// reliance on memory mapped I/O, this implementation is basically unusable on
// 32-bit systems (filesystem size must be less than 4GiB).

static mut SHOULD_CRASH: atomics::AtomicBool = atomics::INIT_ATOMIC_BOOL;

fn block_size(len: u64) -> u64 {
    (len + (len % 4096)) / 4096
}

fn main() {
    let args = std::os::args();

    let progname = args[0].clone();

    let opts = [
        optopt("d", "disk", "file to use for backing store of the filesystem", "[diskfile]"),
        optopt("f", "format", "size to use for the fileysystem", "[size]"),
        optflag("n", "no-crash", "disable random crashing"),
        optflag("h", "help", "print this message")
    ];

    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => fail!(f.to_err_msg())
    };

    if matches.opt_present("h") {
        println!("{}", usage(progname, opts));
        return
    }

    let disk = Path::new(match matches.opt_str("d") {
        Some(d) => d,
        None => {
            println!("{}", usage(progname, opts));
            fail!("must specify the disk file")
        }
    });

    let size: Option<u64> = matches.opt_str("f").and_then(|s| from_str(s));

    match size {
        Some(size) => {
            if size > (std::uint::MAX as u64) {
                fail!("requested size too large! asked for {}, but can only provide up
                      to {} due to internal limitations", size, std::uint::MAX);
            }
            fs::format(&disk, size);
        },
        None => { }
    }


    // Spawn the "crasher" thread
    native::task::spawn(proc() {
        let soon = 1.0 + (50.0 * rand::random::<f64>() );

        println!("going to crash in {} seconds", soon);
        std::io::timer::sleep(soon as u64 * 1000);

        unsafe { SHOULD_CRASH.store(true, atomics::SeqCst); }
    });

    // Herp the derp
    let derp = fs::mount(&disk);
    // Mount it for real (starts the fuse daemon)
    fuse::mount(derp, &disk, []);
}
