//! On-disk layout

use std;

type Offset = u64;

/// Describes the filesystem
pub struct DerpMetadata {
    /// "derpfs!!"
    magic: [u8, ..8],
    uuid: [u8, ..16],
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
    id_map: Offset,
    string_map: Offset,
    free_map: Offset,
    root_elist: Offset,

}

pub struct EntityListHeader {
    id: u64,
    owner: u64,
    group: u64,
    perm: u16,
    flags: u16,
    _reserved0: u32,
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
    // not hurt by padding
    assert_eq!(std::mem::size_of::<DerpMetadata>(), 64);
    assert_eq!(std::mem::size_of::<EntityListHeader>(), (8 * 7));
}
