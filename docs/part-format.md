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

The writer commits one chunk at a time:

1. write chunk bytes at the content offset
2. `sync_data`
3. mark the chunk complete in memory
4. write the next metadata slot generation
5. `sync_all`

This ordering means a crash can cause a recently written chunk to be downloaded
again, but cannot mark missing data as complete.
