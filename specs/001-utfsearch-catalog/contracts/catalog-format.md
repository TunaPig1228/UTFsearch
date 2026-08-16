# Contract: Catalog file (`*.uts`)

Little-endian. Version 1.

```
offset 0   magic            b"UTFS"
offset 4   format_version   u16   (= 1)
offset 6   flags            u16
offset 8   built_at         i64
offset 16  entry_count      u64
offset 24  section_count    u32
offset 28  header_crc       u32
offset 32  section_table    [ { id:u32, offset:u64, length:u64, crc:u32 } ; section_count ]
```

Required sections (ids stable):

| id | name | contents |
| --- | --- | --- |
| 1 | roots | packed Root records + path bytes |
| 2 | intern | FST bytes (component → u32) |
| 3 | entries | fixed-width Entry records, `entry_count` long |
| 4 | trigrams | trigram (u32 packed 3×u21 or 3×char32) → roaring blob |
| 5 | dir_stats | dir_entry_id + mtime + child_count |
| 6 | meta | JSON or packed status (counts, host, case policy) |

Readers MUST reject unknown required bits in `flags`, reject version > supported, reject CRC mismatch. Extra section ids MAY be ignored.

Atomic publish: write `name.uts.tmp`, fsync file, fsync dir if the OS allows, rename over `name.uts`.

Permissions: Unix `0600`. Windows: inherit user-only DACL as far as `std`/`windows-sys` allow without pulling a huge crate; document residual risk.

Readers map the file read-only (`memmap2`). Writers never mutate a mapped complete file in place.
