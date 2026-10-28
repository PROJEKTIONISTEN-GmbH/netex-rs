# Reading the German delivery fast: what was measured, what was changed

The DELFI delivery is 28 176 line files, 77.9 GB of XML inflated from a 2.0 GB
zip (deflate, 39:1). Everything below was measured on the delivery of
2026-09-21, release build, `RAYON_NUM_THREADS` threads, on a host shared with
other work — single runs vary by some ten per cent; the load average at the
start of each run is in its log. Two containers were used: one with a quota of
**32 CPUs** and 24 GiB, where the curves up to 32 threads were measured, and
one with **8 CPUs** and 16 GiB, where the final stand was measured up to 8
threads, beside the stand before this work on the same machine. More than 32
threads were not measurable. The harness that split the time by phase and
drew the profiles lives outside the crate (a scratch binary over `zip`,
`quick-xml`, `pprof`); `examples/index_delivery.rs` is the measurement that
ships.

## Before: the curve knee and its cause

32-CPU container. `examples/index_delivery.rs` as it was — rayon over the archive's entries in
archive order, one archive handle per rayon *job*, every document read into the
full typed model, one `IndexBuilder` under a mutex:

| threads | read + gather | MB/s | MB/s per thread | total | peak RSS |
|---|---|---|---|---|---|
| 1 | 644 s | 121 | 121 | 649 s | 12.5 GB |
| 2 | 330 s | 236 | 118 | 335 s | 10.9 GB |
| 4 | 178 s | 438 | 110 | 182 s | 11.1 GB |
| 8 | 115 s | 679 | 85 | 119 s | 11.6 GB |
| 16 | 112 s | 697 | 44 | 116 s | 14.8 GB |
| 24 | 212 s | 367 | 15 | 217 s | 14.3 GB |
| 32 | 154 s | 506 | 16 | 161 s | 17.2 GB |

Up to four threads the read scales; from eight on it flattens, and beyond
sixteen more threads make it slower. Split by phase on 32 threads, summed over
the threads: inflate 47 s, tokenizing 134 s, typed reading 543 s (tokenizing
included), waiting for the builder's mutex 185 s, gathering under it 59 s —
969 thread-seconds of work in 5 238 available. The threads were **not** in the
reading closure four fifths of the time, and it was no straggler tail: every
thread finished its last file in the same second.

The four fifths went into `map_init`: rayon split the 28 176 entries into
7 671 jobs, and each job opened the archive anew. `zip::ZipArchive::new` reads
the central directory of 29 549 entries through an unbuffered `File` —
0.15 s alone, 0.52 s when 32 threads do it at once — and 7 671 × 0.52 s is
4 025 thread-seconds. More threads, more stealing, more jobs, more openings:
that is the knee. (The pprof profile of the same run shows a fifth of the
sampled CPU in `ZipArchive::new` → `read_central_header`.) Second, behind it,
the one mutex: 185 s of waiting plus 59 s of serial gathering is an Amdahl
floor of about a minute whatever the thread count.

Not the cause: inflate (5 % of the work; zlib-rs through the `zip` crate does
1.5 GB/s per thread, libdeflate would do 4 GB/s — measured on 74 real entries —
and save about 30 thread-seconds of 700, not worth a C dependency yet); memory
bandwidth or the allocator (typed reading runs at 135–143 MB/s per thread on
32 threads, as on one); the cgroup's CPU quota (87 s throttled of 5 000 in the
32-thread run).

## After: what changed

- **One parsed archive.** `Source::open` parses the central directory once;
  every worker clones the parsed archive over a `SharedFile`, a file read by
  position (`pread`), so there is no per-job opening and no lock.
- **Largest first, one queue.** `Source::read_each_with` runs on every thread
  of the rayon pool (`rayon::broadcast`) and pulls entries from one shared
  counter, largest first — a worker that finishes a small file takes the next
  largest one left. `documents()` keeps its parallel-iterator shape and the
  same order.
- **One buffer per worker.** The inflated text borrows a buffer the worker
  keeps from document to document, rather than a `String` per document.
- **One builder per worker, merged at the end.** `IndexBuilder::merge` takes
  another builder's records in — its strings renumbered into the receiving
  interner, its ranges into the flat arrays translated, an entity whose key is
  already in counted as a redefinition exactly as a single builder would —
  and `merge_all` merges sequentially from the largest builder. A parallel
  pairwise merge was tried first: it holds two halves and their union at once
  and reached the container's 24 GiB (memory peak 24.3 GB, `memory.events max`
  fired, merge 48 s); the sequential merge peaks at 16 GB.
- **Each string once.** The interner keeps a string once (a `HashTable` of
  numbers probed by hash, compared against the string's own storage) instead
  of once as a key and once in the list; before a merge the table is given
  room for every worker's strings, so it grows once instead of rehashing
  25 million strings at every doubling.
- **Lean mode.** `stream::read` yields typed entities of selected kinds and
  skips everything else at the tokenizer; `Index::build` reads a dataset that
  way with no document model in memory.

- **Every quay kept, whatever the order.** The line files repeat the stops
  they serve, each with the quays it serves, and a redefined stop place used
  to be left out *with* its quays: a quay only a later file named was lost,
  and which quays were lost depended on the order the files were read in —
  35 186 to 48 909 unresolved `QuayRef`s from run to run and thread count to
  thread count, before and after the parallel read. A redefined stop place now brings its new quays to the place
  that was kept, and where files contradict each other (over the delivery:
  two quays under two stop places, one stop point assigned to two quays) the
  file whose name sorts first decides. Unresolved references: **0**; the
  index is the same on any number of threads, entity for entity.

Allocations per MB of input, one thread, every 20th file of the delivery
(counted by a counting global allocator; before: 3.6 GB, after: 3.9 GB):

| | allocations per MB | MB allocated per MB |
|---|---|---|
| inflate | 2 → 2 | 1.0 → 1.0 |
| full model: read and gather | 30 427 → 24 107 | 6.2 → 5.8 |
| lean (routing profile): read and gather | — → 24 372 | — → 5.4 |

The lean path allocates as much per MB as the full one because the routing
profile types the bulk of every line file — see below.

## After: the curve

Full model, `examples/index_delivery.rs`. Read and gather run on every thread;
the merge of the workers' builders and the resolution run on one.

32-CPU container, stand `019d2be` (the read and merge of this work, before the
quay fix; the fix touches only the redefinition branch, and on the 8-CPU
container the two stands measure the same, 97–104 s on 8 threads each, two runs
apiece):

| threads | read + gather | MB/s | MB/s per thread | merge | resolve | total | peak RSS | before, total |
|---|---|---|---|---|---|---|---|---|
| 1 | 660 s | 118 | 118 | 0 s | 4.7 s | 665 s | 10.5 GB | 649 s |
| 2 | 321 s | 243 | 121 | 10.1 s | 4.7 s | 336 s | 12.5 GB | 335 s |
| 4 | 157 s | 496 | 124 | 12.6 s | 4.6 s | 174 s | 11.9 GB | 182 s |
| 8 | 78.4 s | 994 | 124 | 13.8 s | 4.6 s | 96.8 s | 12.3 GB | 119 s |
| 16 | 44.7 s | 1 743 | 109 | 14.5 s | 4.6 s | 63.8 s | 13.4 GB | 116 s |
| 24 | 28.5 s | 2 734 | 114 | 14.7 s | 4.6 s | 47.8 s | 14.7 GB | 217 s |
| 32 | 24.5 s | 3 185 | 100 | 15.0 s | 4.5 s | **43.9 s** | 16.0 GB | 161 s |

8-CPU container, the final stand, and the stand before this work (`f846744`) on
the same machine:

| threads | read + gather | MB/s | MB/s per thread | merge | resolve | total | peak RSS | before, total |
|---|---|---|---|---|---|---|---|---|
| 1 | 665 s | 117 | 117 | 0 s | 4.8 s | 670 s | 10.5 GB | 646 s |
| 2 | 329 s | 237 | 118 | 10.5 s | 4.8 s | 344 s | 12.6 GB | |
| 4 | 162 s | 481 | 120 | 12.8 s | 4.7 s | 179 s | 11.8 GB | |
| 8 | 83.2 s | 936 | 117 | 14.5 s | 4.5 s | 102 s | 12.2 GB | 119 s |

The read now scales to 32 threads: 100–124 MB/s per thread against 16 before.
On eight cores the gain is small (119 s → 102 s), because the knee sat above
eight; it is above eight cores that the lost parallelism came back. One thread
is about 4 % slower than before on the delivery, on both machines (649 → 665 s,
646 → 670 s), although it is 5 % faster on the 1.1 MB fixture the guards time
(137 → 144 MB/s); the cause was not isolated. The
knee that remains is the serial tail: merging the builders takes 10 s for two
and 15 s for 32 of them, resolving 4.6 s, so on 32 threads 20 of 44 s are spent
on one core. The merge's cost is inserting the ~25 million strings each known
to one worker only (about 240 ns each, bound by memory latency) and moving
100 million passing times; parallel lookups of the strings the builders share
gained nothing measurable. A hash-sharded interner, so that insertion runs on
every core, is the next lever. The 32-thread point also ran beside other
work in the same CPU quota (load 6–17 at the starts), which is why
MB/s per thread dips there.

## The lean path

`Index::build` with `Selection::routing()`, no document model in memory:

| | 1 thread | 8 threads | 16 threads | 32 threads | peak RSS |
|---|---|---|---|---|---|
| 32-CPU container, `019d2be` | 585 s | 90.8 s | 59.6 s | 41.8 s | 13.2 GB on 32 threads |
| 8-CPU container, final | 584 s | 95.7 s | — | — | 12.2 GB on 8 threads |
| 8-CPU container, streamed to a consumer that counts, no index | | 68.9 s | — | — | 0.64 GB |
| 8-CPU container, nothing selected: inflate and tokenize only | | 23.2 s | — | — | 0.62 GB |

Lean is 12 % faster than the full model on one thread and 5 % on 32. The
profile says why (8 threads, `flamegraph-after-lean-8-threads.svg`): of the
samples, 48 % are typing service journeys with their passing times, 29 %
journey patterns with their stop points and notice assignments — what a
routing import needs is the bulk of every line file. Skipping everything else
costs 0.16 %, inflating 3 %. On 8 threads the lean index build splits into
23 s that no selection can save (inflating and tokenizing every byte), 46 s
typing the entities of the profile, and 27 s gathering them into the builders,
merging and resolving. The peak memory is the index's, not the documents':
streamed to a consumer that keeps nothing, the same read peaks at 0.64 GB
(`stream::read` over `Source::read_each`), while the builders of the index
hold 100 million passing times either way.

## Where the time goes now

Full model, 8 threads (`flamegraph-after-full-8-threads.svg`): typed reading
78 % of the samples, of it quick-xml's `read_event` 12 %; inflating into the
worker's buffer 6 %; freeing the documents 3 %; merging 2 %; resolving 1 %.
Opening the archive, which took a fifth of the samples before, is gone from
the profile. (pprof samples by `SIGPROF`; under many threads the signals
coalesce, so the shares hold and the absolute counts do not.)

## Guards

`tests/guards.rs` counts what reading the 1.1 MB fixture allocates (exact, so
the thresholds sit a fifth above today's figures) and times it (best of seven,
floors at a third of today's throughput, release builds only). `cargo bench`
has `read` (full model), `read_lean` (routing profile), `write`, `index`, and
`dataset` (the five fixtures indexed full and lean), one thread each.

## 0.3.0: what answering the questions costs

0.3.0 resolves in the document first, sorts what each document defines by the
documents' names, fingerprints every repeated stop, counts every decision as a
finding and reads calls, availability conditions and template journeys
(`docs/INTERPRETATION.md`). Measured with `examples/index_delivery --lean`, 8
threads, in the development container (8 CPUs, 16 GiB; host load 7 to 21 from
other work), every run under the shared measuring lock, before and after
alternating (A B B A), the process's wall clock and peak RSS, the mean of two
runs each:

| Delivery | XML GB | 0.2.3 | 0.3.0 | time |
|---|---:|---:|---:|---:|
| Linz AG | 0.53 | 648 ms, 159 MB | 699 ms, 159 MB | +7.8 % |
| ÖBB/EVU | 1.04 | 1 186 ms, 230 MB | 1 301 ms, 232 MB | +9.7 % |
| Kärnten | 0.92 | 1 152 ms, 202 MB | 1 306 ms, 203 MB | +13.4 % |
| VOR | 5.29 | 6 290 ms, 1 427 MB | 6 932 ms, 1 424 MB | +10.2 % |
| DELFI, the 1 569 documents of the DB folders | 3.49 | 4 516 ms, 702 MB | 4 784 ms, 677 MB | +5.9 % |
| DELFI, the whole delivery of 2026-10-12 | 75.8 | 93.3 s, 11.72 GB | 95.6 s, 11.68 GB | +2.4 % |
| Swiss part (five shared documents, timetable documents 1 to 20) | 1.41 | 1 372 ms, 860 MB | 2 316 ms, 1 035 MB | +68.8 % |

The Swiss part costs more because 0.3.0 reads what 0.2 skipped: its 715 091
calls with times and its 86 127 availability conditions — 0.2 read its 102 387
journeys without a single stop. Elsewhere the cost is the finish: on the
ÖBB/EVU delivery reading and gathering take 0.92 s and merging 0.2 s either
way, resolving 0.07 s before and 0.14 s after, about 30 ms of that sorting its
139 208 findings (on keys made once and the document by its rank; sorting by
the findings' text took about 50 ms). Peak memory does not grow but where
there is more to hold.

The index of 0.2.3 was not deterministic on deliveries reusing identifiers
per document: its two runs gave two content fingerprints on Linz, Kärnten and
VOR; every run of 0.3.0 gave one.
