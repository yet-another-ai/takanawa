# Takanawa `.part` Format

The `.part` file layout is:

```text
[content bytes, length = content_len]
[metadata slot 0, length = aligned_slot_size]
[metadata slot 1, length = aligned_slot_size]
```

Each metadata slot is fixed-size for a given `content_len` and `chunk_size`.
The active slot is the valid slot with the largest `generation`.

Metadata includes:

- magic and metadata version
- CRC32 over the whole slot with the CRC field zeroed
- generation
- URL SHA-256
- content length, chunk size, chunk count
- completion bitmap
- ETag and Last-Modified snapshots
- expected hash configuration

The writer may stream bytes into the content area before a chunk is complete.
Those bytes are intentionally not durable resume state yet: a chunk is
recoverable only after its bitmap bit is committed in metadata. If a process
crashes while a chunk is partially written, the next run treats that chunk as
incomplete and downloads it again, overwriting any partial content bytes.

The writer commits one completed chunk at a time:

1. write chunk bytes at the content offset
2. `sync_data`
3. mark the chunk complete in memory
4. write the next metadata slot generation
5. `sync_all`

This ordering means a crash can cause a recently written chunk to be downloaded
again, but cannot mark missing data as complete.
