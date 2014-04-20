//! Data layout for the filesystem

use std::cast::transmute;
use std::kinds::marker::InvariantType;

use uuid::Uuid;
use bitmap::Bitmap;

macro_rules! diskstuff (
    ($($name:ty = $size:expr);*) => (
        #[test]
        fn size_of() {
            $(
                assert_eq!(::std::mem::size_of::<$name>(), $size);
            )*
        }

        $( impl Persist for $name { } )*
    )
)

pub trait Persist {
    unsafe fn save(&mut self, location: *mut u8) {
        ::std::ptr::copy_memory(location as *mut Self, self as *mut Self as *Self, 1);
    }
}

pub struct Superblock {
    pub magic: [u8, ..8],
    pub uuid: Uuid,
    pub size: u64,
    pub flags: u64,
    pub freemap: Offset<Bitmap>,
    pub idmap: Offset<IList<IdEntry>>,
    pub strmap: Offset<IList<StrEntry>>,
    pub root: Offset<Entity>
}

pub fn magic() -> [u8, ..8] {
    ['d' as u8, 'e' as u8, 'r' as u8, 'p' as u8, 'f' as u8, 's' as u8, '!' as u8, '!' as u8]
}

impl Superblock {
    pub fn empty() -> Superblock {
        Superblock {
            magic: magic(),
            uuid: Uuid::nil(),
            size: 0,
            flags: 0,
            idmap: Offset::new(0),
            strmap: Offset::new(0),
            freemap: Offset::new(0),
            root: Offset::new(0),
        }
    }
}

pub struct IdEntry {
    pub id: EntityId,
    pub offset: Offset<Entity>
}

pub struct StrEntry {
    pub id: StrId,
    pub len: u64,
    pub loc: Offset<u8>
}

// really not that good, ideally this should be tied to some lifetime.
pub struct Offset<T> {
    pub loc: u64,
    pub marker: InvariantType<T>
}

impl<T> Eq for Offset<T> {
    fn eq(&self, other: &Offset<T>) -> bool {
        self.loc == other.loc
    }
}

impl<T> Offset<T> {
    pub fn new(loc: u64) -> Offset<T> {
        Offset { loc: loc, marker: InvariantType }
    }

    pub fn get<'a>(&'a self, base: *mut u8) -> &'a T {
        unsafe { transmute(base.offset(self.loc as int)) }
    }

    pub fn get_mut<'a>(&'a mut self, base: *mut u8) -> &'a mut T {
        unsafe { transmute(base.offset(self.loc as int)) }
    }
}

/// An "Inline List". The contents of the list immediately follow it.
pub struct IList<T> {
    pub next: Offset<IList<T>>,
    pub len: u64,
}

/// In iterator over an IList's elements
pub struct IListElements<'a, T> {
    base: *mut u8,
    current: &'a IList<T>,
    loc: uint,
}

impl<'a, T> Iterator<&'a T> for IListElements<'a, T> {
    fn next(&mut self) -> Option<&'a T> {
        if self.loc as u64 > (self.current.len - 1) {
            if self.current.next == Offset::new(0) {
                None
            } else {
                self.loc = 1;
                self.current = self.current.next.get(self.base);
                unsafe { transmute(self.current.get(0)) }
            }
        } else {
            self.loc += 1;
            unsafe { transmute(self.current.get(self.loc - 1)) }
        }
    }
}

impl<T> IList<T> {
    pub fn empty() -> IList<T> {
        IList {
            next: Offset::new(0),
            len: 0
        }
    }

    pub fn iter<'a>(&'a self, base: *mut u8) -> IListElements<'a, T> {
        IListElements {
            base: base,
            current: self,
            loc: 0
        }
    }

    pub fn get<'a>(&'a self, idx: uint) -> Option<&'a T> {
        if idx as u64 > self.len {
            None
        } else {
            unsafe {
                let s = (self as *IList<T>).offset(1) as *T;
                Some(transmute(s.offset(idx as int)))
            }
        }
    }

    pub fn get_mut<'a>(&'a mut self, idx: uint) -> Option<&'a mut T> {
        if idx as u64 > self.len {
            None
        } else {
            unsafe {
                let s = (self as *mut IList<T>).offset(1) as *mut T;
                Some(transmute(s.offset(idx as int)))
            }
        }
    }
}

pub struct Entity {
    pub id: EntityId,
    pub parent: EntityId,
    pub size: u64,
    pub owner: u64,
    pub group: u64,
    pub perm: u32,
    pub flags: u32,
    pub attrs: Offset<u8>,
    pub chunks: IList<ContentChunk>,
}

impl Entity {
    /// Return an iterator over this Entity's chunks, or None if all of its
    /// content is inline.
    pub fn chunks<'a>(&'a self, base: *mut u8) -> Option<IListElements<'a, ContentChunk>> {
        if self.flags & (1 << 1) != 0 {
            None
        } else {
            Some(self.chunks.iter(base))
        }
    }
}

pub struct ContentChunk {
    pub len_or_name: u64,
    pub offset: Offset<u8>
}

diskstuff!(
    Superblock   = 72;
    Entity       = 72;
    StrEntry     = 24;
    IdEntry      = 16;
    IList<u64>    = 16;
    Offset<u8>   = 8;
    ContentChunk = 16
)

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

opaque!(EntityId)
opaque!(StrId)
