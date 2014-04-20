Filesystem submission for my OS course at Clarkson.

Design
======

The following is a graphviz description of the disk layout::

    digraph fs {
        node [shape = record];
        rankdir = LR;
        ranksep = 1;

        superblock [ label = "magic : u64 | uuid : u128 | size : u64 | flags : u64 | <id> idmap : Offset | <str> strmap : Offset | <free> freemap : Offset | <root> Root Element : Offset" ];

        freemap [ label = "<start> ...  | { Multiply Referenced : bit | Used : bit } | ... " ];

        idmap [ label = "<start> next : Offset | len : u64 | ... | {id : Id | Offset} | ..." ];

        strmap [ label = "<start>next : Offset | len : u64  | ... | {id : StrId | len : u64 | loc : Offset } | ..."];

        entity [ label = "<start> id : u64 | size : u64 | owner : u64 | group : u64 | { perm : u32 | flags : u32 } | attrs : Offset | parent : Id | attrs : Offset | <next> next : Offset | ... | { name : StrId / len : u64 | offset : Offset } | ... "];

        conchunk [ label = "<start> next : Offset | len : u64 | ... | { name : StrId / len : u64 | offset : Offset } | ... "];

        superblock:id -> idmap:start;
        superblock:str -> strmap:start;
        superblock:free -> freemap:start;
        superblock:root -> entity:start;

        entity:next -> conchunk:start [ weight = 100 ];
    }

(Rendered: http://i.imgur.com/ScNlxeN.png)

The superblock contains the size of the filesystem, in bytes aligned to a 4K
block. It assumes the backing store is contiguous.

The freemap is a dense bitmap, two bits per block, signifying the number of
inodes that reference that block. If it is 0, it is free and may be allocated.
If it is 1, there is only one reference to that block, and it may be set to 0
(free) once its owner is removed. If it is 2, there are 2 references to it,
and it may be decremented when either of the references are removed. If it is
3, there was at one point more than 2 references to the block at a single
time. It can never be decremented except by rebuilding the freemap from the
current state of the filesystem. Its size can be large for large filesystems.
For a 4TiB filesystem, it will be 256MiB. Thus, it should only be used through
memory-mapped I/O, and avoid living in real memory.

The strmap, conchunk, and idmap all share a common structure. They are a form
of linked list, consisting of the offset of the next chunk of the list, the
length of the current chunk, and then the contents of the chunk. These are
meant to be a low-overhead encoding of a map. For speed, the filesystem
implementation uses a hash map from Id to Offset and from StrId to a tuple of
the string's length and its offset.

In general, every object is at least one page in size. Thus the superblock,
while only being only 72 bytes long in the current revision, has an entire
block to itself. This is especially useful for small files. Since the inode
takes an entire block, the contents of a small file can use the rest of the 4K
that is not taken up by the inode. When the file grows larger, the inode will
have space for many spans available to store the extent of the file's contents
without needing to spill into a conchunk.

The length field is not a single byte length, but rather split into a block
length and a byte length. Thus a span can have much more unused space after
its actual byte length that in can still use, besides just the amount of space
to the furthest 4K boundary. With 32 bits for the blocks, we get that we can
have a 16TiB span. This is insane, since the byte length could only be 2^32,
or 4GiB. Due to this inequality (block length is multiplied by 4K), we can
solve for this equation to get the ideal number of bits for the block length:

.. math::

    2^x * 4096 = 2^{64 - x}

Thankfully this has an integer solution: 26. With 26 bits for the block length
and 38 for the byte length, each can cover 256GiB.
