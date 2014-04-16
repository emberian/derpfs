use std::os::{MemoryMap, MapReadable, MapWritable, MapFd};
use std::io::{BufWriter, TypeFile, TypeDirectory};
use std::cast::{forget, transmute};
use std::iter::range_inclusive;
use std::mem::size_of;

use block_size;
use mk_slice;
use Offset;
use StrId;
use disk;
use Id;

use collections::HashMap;
use bitmap::Bitmap;
use uuid::Uuid;
use libc;

use fuse::{Request, ReplyDirectory, ReplyEntry, ReplyEmpty, ReplyOpen, ReplyData, ReplyWrite};
use fuse;

// NB: explicitly not Clone. Cloning this is a bug.
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

#[deriving(Clone, Show)]
pub struct Entity {
    id: Id,
    parent: Id,
    owner: u64,
    group: u64,
    attrs: u64,
    length: u64,
    perms: u32,
    flags: EntFlags,
    contents: ~[ConChunk],
    children: ~[DirEnt],
}

#[deriving(Clone, Eq, Show)]
pub struct EntFlags(u32);

impl EntFlags {
    pub fn new() -> EntFlags {
        EntFlags(0)
    }

    pub fn is_dir(&self) -> bool {
        let EntFlags(f) = *self;
        (f & 0b1) != 0
    }

    pub fn set_dir(&mut self) {
        let &EntFlags(ref mut f) = self;
        *f |= 0b1;
    }

    pub fn clear_dir(&mut self) {
        let &EntFlags(ref mut f) = self;
        *f &= !0b1;
    }
}

impl Entity {
    fn new_raw() -> Entity {
        Entity {
            id: Id::new(0),
            parent: Id::new(0),
            owner: 0,
            group: 0,
            flags: EntFlags::new(),
            attrs: 0,
            length: 0,
            contents: ~[],
            children: ~[],
            perms: 0,
        }
    }
}

#[deriving(Clone, Eq, Show)]
pub struct ConChunk {
    addr: Offset,
    len: u64,
}

#[deriving(Clone, Eq, Show)]
pub struct DirEnt {
    name: StrId,
    id: Id,
}

extern {
    fn ioctl(fd: libc::c_int, req: libc::c_int, ...) -> libc::c_int;
}

static BLKGETSIZE64: libc::c_int = 2148012658;

fn bytes_in_blockdev(p: &Path) -> Option<u64> {
    p.with_c_str(|path| {
        let mut size: u64 = 0;
        let fd = unsafe { libc::open(path, libc::O_RDONLY, libc::S_IRUSR | libc::S_IWUSR) };
        let ret = unsafe { ioctl(fd, BLKGETSIZE64, &mut size as *mut u64) };
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
        let size = bytes_in_blockdev(path).unwrap();
        /* we might be losing a few bytes at the end if it's using strange
         * block size, but whatever */
        let bitmap = Bitmap::new(2, (size / 4096) as uint).unwrap();
        let fd = path.with_c_str(|path| {
            let ret = unsafe { libc::open(path, libc::O_RDWR, libc::S_IRUSR | libc::S_IWUSR) };
            if ret == -1 {
                None
            } else {
                Some(ret)
            }
        });
        if fd == None { return None }
        let fd = fd.unwrap();

        let map = MemoryMap::new(size as uint, [MapReadable, MapWritable, MapFd(fd)]).unwrap();
        let mut idmap = HashMap::new();
        let strmap = HashMap::new();
        let mut root = Entity::new_raw();
        root.id = Id::new(1);
        root.perms = 0b111_101_101; // 0755

        let mut fs = Filesystem {
            uuid: uuid,
            root: root,
            blockmap: bitmap,
            ids: idmap,
            strmap: strmap,
            disk: map,
            meta: disk::Metadata {
                size: size,
                flags: 0,
                num_ids: 1,
                id_map: Offset::new(0),
                num_strings: 0,
                string_map: Offset::new(0),
                free_map: Offset::new(0),
                root: Offset::new(0)
            },
        };
        fs.create();
        Some(fs)
    }

    /// Create the filesystem on disk
    fn create(&mut self) {
        static MAGIC: &'static [u8] = bytes!("derpfs!!");

        let mut buf = unsafe { mk_slice(self.disk.data, 0, self.meta.size as uint) };

        let mut wr = BufWriter::new(buf);

        let mut blockpos = 4096; // first 4K reserved for superblock

        // leave the superblock alone.
        self.blockmap.set(0, 0b01);

        // write out the superblock
        wr.write(MAGIC);
        wr.write(self.uuid.as_bytes());

        // how long are the maps going to be?
        let bitmap_size = block_size(self.blockmap.byte_len());
        // add this because we optionally store an offset to the next "chunk"
        // of the map, and the length of this chunk
        let overhead = size_of::<u64>() * 2;
        let idmap_size = block_size(self.ids.len() * (size_of::<Id>() + size_of::<Offset>()) + overhead);
        let strmap_size = block_size(self.strmap.len() * (size_of::<StrId>() + size_of::<Offset>()) + overhead);

        // mark them used
        for i in range_inclusive(1, bitmap_size + idmap_size + strmap_size) {
            self.blockmap.set(i as uint, 0b01);
        }

        self.meta.id_map = Offset::new(2);
        self.meta.string_map = Offset::new(2 + idmap_size as u64);
        self.meta.free_map = Offset::new(2 + idmap_size + strmap_size);
        self.meta.root = Offset::new(2 + idmap_size + strmap_size + bitmap_size);
        self.meta.flags = 1 << 63; // "dirty"

        self.meta.save(&mut wr);
    }

    pub fn save(&mut self) {

    }

    pub fn string<'a>(&'a self, id: StrId) -> Option<&'a str> {
        Some("foo")
    }

    pub fn inode(&self, id: Id) -> Option<Entity> {
        Some(Entity::new_raw())
    }
}

impl fuse::Filesystem for Filesystem {
    fn destroy(&mut self, _req: &Request) {
        self.save();
        self.meta.flags &= !(1 << 63); // mark clean
        let sl = unsafe { mk_slice(self.disk.data, 24, self.meta.size as uint) };
        self.meta.save(&mut BufWriter::new(sl));
    }

    fn opendir(&mut self, _req: &Request, inode: u64, _flags: uint, reply: ReplyOpen) {
        // store the current state of the inode, so further modifications
        match self.inode(Id::new(inode)) {
            None => reply.error(libc::ENOENT),
            Some(e) => {
                let ent: ~Entity = ~e.clone();
                let ptr = &*ent as *Entity as uint;
                reply.opened(ptr as u64, 0);
                unsafe { forget(ent); }
            }
        }
    }

    fn readdir(&mut self, _req: &Request, inode: u64, fh: u64, offset: u64, mut reply: ReplyDirectory) {
        let ent: &Entity = unsafe { transmute(fh) };
        info!("readdir: inode={}, fh={}, offset={}", inode, fh, offset);
        debug!("ent: {}", ent);
        if inode != ent.id.val() {
            error!("inode and fh disagree!");
            reply.error(libc::EBADF);
            return
        }

        let mut error = None;

        for (off, dirent) in ent.children.iter().enumerate().skip(offset as uint) {
            match self.inode(dirent.id) {
                None => {
                    error!("corruption! id {}, listed in the dirent for {}, does not exist", dirent.id, ent.id);
                    error = Some(libc::EBADF);
                    break;
                },
                Some(d) => {
                    let kind = if d.flags.is_dir() { TypeDirectory } else { TypeFile };
                    if reply.add(inode, off as u64, kind, &PosixPath::new(self.string(dirent.name).unwrap())) {
                        error = Some(libc::EINVAL);
                        break;
                    }
                }
            }
        }
        match error {
            Some(e) => reply.error(e),
            None    => reply.ok()
        }
    }

    fn releasedir(&mut self, _req: &Request, _inode: u64, fh: u64, _flags: uint, reply: ReplyEmpty) {
        unsafe { transmute::<u64, ~Entity>(fh); }
        reply.ok()
    }
}
