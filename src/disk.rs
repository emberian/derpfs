//! On-disk layout

use std;

use Offset;
use StrId;
use Id;

pub struct Magic {
    /// "derpfs!!"
    pub magic: [u8, ..8],
    pub uuid: [u8, ..16],
}

/// Describes the filesystem
pub struct Metadata {
    pub size: u64,
    /// --MS--
    /// 2: state (00 = clean, 01 = dirty, 10 = error, 11 = recover
    /// 2: error handling (00 = ignore, 01 = remount ro, 10 = bail, 11 = log + ignore)
    /// 4: revision
    /// 1: cow?
    /// 1: dedup?
    /// 1: journal?
    /// .
    /// .
    /// .
    /// --LS--
    pub flags: u64,
    pub num_ids: u64,
    pub id_map: Offset,
    pub num_strings: u64,
    pub string_map: Offset,
    pub free_map: Offset,
    pub root: Offset,
}

impl Metadata {
    pub fn save(&self, wr: &mut std::io::BufWriter) {
        wr.write_le_u64(self.size);
        wr.write_le_u64(self.flags); // no flags
        wr.write_le_u64(self.num_ids);
        wr.write_le_u64(self.id_map.val());
        wr.write_le_u64(self.num_strings);
        wr.write_le_u64(self.string_map.val());
        wr.write_le_u64(self.free_map.val());
        wr.write_le_u64(self.root.val());
    }
}

pub struct EntityListHeader {
    pub id: u64,
    pub owner: u64,
    pub group: u64,
    pub perm: u32,
    pub flags: u32,
    pub attrs: V<u64>,
    pub dirlen: V<u64>,
    pub conlen: V<u64>,
    // dirlen bytes of DirEnts
    // conlen bytes of ConEnts
}

pub struct DirEnt {
    pub name: VStr,
    pub id: u64
}

pub struct ConEnt {
    pub addr: V<u64>,
    pub len: V<u64>,
}

/// A string reference. If the MSB is set, then the remaining 7 bytes are the
/// name of the entity. Else, the remaining 63 bits are the String ID in the
/// string map.
pub struct VStr(Offset);

// TODO: variable length encode/decode
pub struct V<T> {
    data: T
}

#[test]
fn size_of_types() {
    // not hurt by padding etc
    assert_eq!(std::mem::size_of::<DerpMetadata>(), 64);
    assert_eq!(std::mem::size_of::<EntityListHeader>(), (8 * 7));
}
