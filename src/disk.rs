//! On-disk layout

use std;

use Offset;
use StrId;
use Id;

pub struct Magic {
    /// "derpfs!!"
    magic: [u8, ..8],
    uuid: [u8, ..16],
}

/// Describes the filesystem
pub struct Metadata {
    size: u64,
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
    flags: u64,
    num_ids: u64,
    id_map: Offset,
    num_strings: u64,
    string_map: Offset,
    free_map: Offset,
    root: Offset,
}

impl Metadata {
    pub fn save(&self, &mut std::io::BufWriter) {
        wr.write_le_u64(self.size);
        wr.write_le_u64(self.flags); // no flags
        wr.write_le_u64(self.num_ids)
        wr.write_le_u64(self.id_map.val());
        wr.write_le_u64(self.num_strings);
        wr.write_le_u64(self.string_map.val());
        wr.write_le_u64(self.free_map.val());
        wr.write_le_u64(self.root.val());
    }
}

pub struct EntityListHeader {
    id: u64,
    owner: u64,
    group: u64,
    perm: u32,
    flags: u32,
    attrs: V<u64>,
    dirlen: V<u64>,
    conlen: V<u64>,
    // dirlen bytes of DirEnts
    // conlen bytes of ConEnts
}

pub struct DirEnt {
    name: VStr,
    id: u64
}

pub struct ConEnt {
    addr: V<u64>,
    len: V<u64>,
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
