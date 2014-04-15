use std::os::{MemoryMap, MapReadable, MapWritable, MapFd};
use std::iter::range_inclusive;
use std::mem::size_of;

use bitmap::Bitmap;
use uuid::Uuid;
use wsize;
use libc;

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

pub struct Filesystem {
    uuid: Uuid,
    root: Entity,
    blockmap: Bitmap,
    ids: HashMap<Id, Offset>,
    strmap: HashMap<StrId, Offset>,
    meta: disk::Metadata,

    // memory mapped i/o yay!
    disk: MemoryMap,
}

pub struct Entity {
    id: Id,
    owner: u64,
    group: u64,
    attrs: u64,
    length: u64,
    perms: u32,
    flags: u32,
    contents: ~[ConChunk],
    children: ~[DirEnt],
}

impl Entity {
    fn new_raw() -> Entity {
        Entity {
            id: Id::new(0),
            owner: 0,
            group: 0,
            flags: 0,
            attrs: 0,
            length: 0,
            contents: ~[],
            children: ~[],
            perms: 0
        }
    }
}

pub struct ConChunk {
    addr: Offset,
    len: u64,
}

pub struct DirEnt {
    name: StringId,
    id: Id,
}

extern {
    fn ioctl(fd: libc::c_int, req: libc::c_int, ...);
}

static BLKGETSIZE64: c_uint = 2148012658;

fn bytes_in_blockdev(p: &Path) -> Option<u64> {
    p.with_c_str(|path| {
        let mut size: u64 = 0;
        let fd = libc::open(path, libc::O_RDONLY);
        let ret = unsafe { libc::ioctl(fd, BLKGETSIZE64, &mut size as *mut u64); };
        unsafe { libc::close(fd); }
        if ret == -1 {
            None
        } else {
            Some(size)
        }
    })
}

impl Filesystem {
    // TODO: not use the entire file
    pub fn mkfs(path: &Path) -> Option<Filesystem> {
        let uuid = Uuid::new_v4();
        let size = bytes_in_blockdev(path);
        /* we might be losing a few bytes at the end if it's using strange
         * block size, but whatever */
        let bitmap = Bitmap::new(2, size / 4096);
        let fd = path.with_c_str(|path| {
            let ret = unsafe { libc::open(path, libc::O_RDWR) };
            if ret == -1 {
                None
            } else {
                Some(ret)
            }
        });
        if fd == None { return None }
        let fd = fd.unwrap();

        let map = MemoryMap::new(size, [MapReadable, MapWritable, MapFd(fd)]).unwrap();
        let mut idmap = HashMap::new();
        let strmap = HashMap::new();
        let mut root = Entity::new_raw();
        root.id = Id::new(1);
        root.perms = 0b111_101_101; // 0755

        let mut fs = Filesystem {
            uuid: uiud,
            root: root,
            blockmap: bitmap,
            ids: idmap,
            strmap: strmap,
            size: size,
            disk: map
        };
        fs.create();
        Some(fs)
    }

    /// Create the filesystem on disk
    fn create(&mut self) {
        static MAGIC: &'static [u8] = bytes!("derpfs!!");

        let mut buf: &mut [u8] = unsafe { std::cast::transmute( std::raw::Slice { data: self.map.data as *u8, len: self.size } ) };

        let mut wr = std::io::BufWriter::new(buf);

        let mut blockpos = 4096; // first 4K reserved for superblock

        // leave the superblock alone.
        self.blockmap.set(0, 0b01);

        // write out the superblock
        wr.write(MAGIC);
        wr.write(self.uuid.as_bytes());
        wr.write_le_u64(0); // flags NYI

        // how long are the maps going to be?
        let bitmap_size = block_size(self.bitmap.byte_len(), 4096);
        // add this because we optionally store an offset to the next "chunk"
        // of the map, and the length of this chunk
        let overhead = size_of::<u64>() * 2;
        let idmap_size = block_size(self.ids.len() * (size_of::<Id>() + size_of::<Offset>()) + overhead, 4096);
        let strmap_size = block_size(self.strmap.len() * (size_of::<StrId>() + size_of::<Offset>()) + overhead, 4096);

        // mark them used
        for i in range_inclusive(1, bitmap_size + idmap_size + strmap_size) {
            self.bitmap.set(i, 0b01);
        }

        // save all that
        wr.write_le_u64(self.ids.len());
        wr.write_le_u64(2);
        wr.write_le_u64(self.strmap.len());
        wr.write_le_u64(2 + idmap_size);
        wr.write_le_u64(2 + idmap_size + strmap_size);
    }
}
