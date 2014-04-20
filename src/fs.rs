//! Filesystem implementation

use std::io::{TypeFile, TypeBlockSpecial, File, IoError, FileNotFound};
use std::os::{MemoryMap, MapReadable, MapWritable, MapFd};
use std::cast::transmute;

use collections::HashMap;
use bitmap::Bitmap;
use uuid::Uuid;

use libc;
use fuse;

use block_size;
use disk::*;

/// size of block device in 512-byte sectors
static BLKGETSIZE: libc::c_int = 4704;

extern {
    fn ioctl(fd: libc::c_uint, req: libc::c_uint, ...) -> libc::c_int;
}

fn open(p: &Path) -> libc::c_int {
    p.with_c_str(|cstr| unsafe { libc::open(cstr, libc::O_RDWR, libc::S_IRUSR | libc::S_IWUSR) })
}

// Q: Where's all the data at? All I see are offsets!
//
// A: Indeed. By the magic of memory mapped i/o, and the Offset type, we can
// operate entirely as if i/o does not exist! This is quite exciting, because
// it makes the code really easy to write, but also unfortunate, because when
// it comes time to port this filesystem to something that isn't userspace,
// it's going to have a lot harder time. That's solvable by storing a map from
// EntityId to actual Entity too, though. See Offset::get and Offset::get_mut.
// Notice that they take a base -- this comes from the MemoryMap.
pub struct DerpFS {
    sb: Superblock,
    map: MemoryMap,
    ids: HashMap<EntityId, Offset<u8>>,
    strs: HashMap<StrId, (u64, Offset<u8>)>,
}

fn map(p: &Path, size: uint) -> MemoryMap {
    let fd = open(p);
    let map = MemoryMap::new(size as uint, [MapReadable, MapWritable, MapFd(fd)]).unwrap();
    unsafe { libc::close(fd); }
    map
}

pub enum OpenError {
    InvalidMagic([u8, ..8]),
    NilUuid,
}

impl DerpFS {
    fn empty(path: &Path, size: u64) -> DerpFS {
        DerpFS {
            sb: Superblock::empty(),
            map: map(path, size as uint),
            ids: HashMap::new(),
            strs: HashMap::new()
        }
    }

    pub fn format(path: &Path, size: u64) -> DerpFS {
        let mut derp = DerpFS::empty(path, size);
        let mut first = 0;
        derp.sb.uuid = Uuid::new_v4();
        derp.sb.size = size;
        derp.sb.flags |= 1 << 0; // "dirty"
        // make the bitmap
        let num_blocks = block_size(size);
        let mut bm = unsafe { Bitmap::new_raw(num_blocks as uint, 2, derp.map.data) }.unwrap();
        // mark the bitmap and superblock as used
        bm.set(0, 0b1); // superblock is always a whole block
        first += 1;
        derp.sb.freemap = Offset::new(first);
        let bitmap_blocks = block_size(bm.byte_len() as u64) + 1;

        for i in range(first as uint, bitmap_blocks as uint) {
            assert!(bm.set(i, 0b1));
        }

        first += bitmap_blocks;
        derp.sb.idmap = Offset::new(first);

        // reserve 10M each (2560 4K blocks) for the idmap and the stringmap

        for i in range(first as uint, first as uint + 2560) {
            assert!(bm.set(i, 0b1));
        }

        first += 2560;
        derp.sb.strmap = Offset::new(first);

        for i in range(first as uint, first as uint + 2560) {
            assert!(bm.set(i, 0b1));
        }

        first += 2560;
        derp.sb.root = Offset::new(first);

        // make the root entity
        {
            let ent = derp.sb.root.get_mut(derp.map.data);

            let root = Entity {
                id: EntityId::new(1),
                parent: EntityId::new(1),
                size: 0,
                owner: 0,
                group: 0,
                perm: 0x1ED,
                flags: 0x1, // is a dir
                attrs: Offset::new(0),
                chunks: IList::empty()
            };

            derp.ids.insert(EntityId::new(1), Offset::new(first));

            *ent = root;
        }
        // empty out the string map and idmap
        {
            let strmap = derp.sb.strmap.get_mut(derp.map.data);
            strmap.next = Offset::new(0);
            strmap.len = 0;
        }

        {
            let idmap = derp.sb.strmap.get_mut(derp.map.data);
            idmap.next = Offset::new(0);
            idmap.len = 0;
        }

        // we have ourselves a filesystem.
        derp
    }

    pub fn open(p: &Path) -> Result<DerpFS, OpenError> {
        let map = map(p, 1024);
        let sb: &Superblock = unsafe { transmute(map.data) };

        if sb.magic != magic() {
            return Err(InvalidMagic(sb.magic))
        } else if sb.uuid == Uuid::nil() {
            return Err(NilUuid)
        }

        let size = sb.size;
        drop(map);

        let mut derp = DerpFS::empty(p, size);
        {
            let sb: &mut Superblock = unsafe { transmute(derp.map.data) };
            derp.sb = *sb;
        }

        Ok(derp)
    }
}

impl fuse::Filesystem for DerpFS { }

fn blockdev_size(p: &Path) -> Option<uint> {
    let fd = open(p);
    let mut size: i32 = 0;
    let ret = unsafe { ioctl(fd as u32, BLKGETSIZE as u32, &mut size as *mut i32) };
    unsafe { libc::close(fd); }

    if ret == -1 {
        None
    } else {
        Some(size as uint * 512)
    }
}

/// Check if a given path could be a filesystem. Ok if there are no problems
/// found (true if it exists, false if it needs to be created).
fn verify(p: &Path, size: u64) -> Result<bool, ~str> {
    let st = p.stat();

    match st {
        Ok(st) => {
            match st.kind {
                TypeFile => {
                    if size > st.size {
                        Err(format!("file not large enough to format; file is {} bytes, requested {}", st.size, size))
                    } else {
                        Ok(true)
                    }
                },
                TypeBlockSpecial => {
                    match blockdev_size(p) {
                        Some(s) => {
                            if size > s as u64 {
                                Err(format!("block device too small, asked for {} bytes but there are only {}", size, s))
                            } else {
                                Ok(true)
                            }
                        },
                        None => Err(format!("some error in the ioctl :("))
                    }
                },
                _ => Err(format!("cannot make a filesystem on {}, which isn't a file or block device", p.display()))
            }
        },
        Err(IoError { kind: FileNotFound, .. }) => {
            Ok(false)
        },
        Err(e) => Err(format!("couldn't stat {}: {}", p.display(), e))
    }
}

pub fn format(p: &Path, size: u64) {
    match verify(p, size) {
        Ok(true) => { },
        Ok(false) => {
            // make the file
            let mut f = File::create(p).unwrap();
            f.truncate(size as i64).unwrap();
        },
        Err(msg) => fail!(msg)
    }

    DerpFS::format(p, size);
}

pub fn mount(p: &Path) -> DerpFS {
    match verify(p, 0) {
        Ok(true) => DerpFS::open(p).ok().unwrap(),
        Ok(false) => fail!("cannot mount non-existent file, format first"),
        Err(e) => fail!("io error: {}", e)
    }
}
