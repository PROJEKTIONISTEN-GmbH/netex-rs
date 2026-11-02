# Performance

What reading a NeTEx delivery and converting it to GTFS costs: first what
0.9.0 changed in writing the feed, then whole national deliveries as of
0.8.1, read and converted to GTFS, each feed checked against the GTFS its
publisher writes from the same data; then how reading the German delivery got
fast, in 0.2 and 0.3.

## Writing the feed on every thread (0.9.0)

netex2gtfs 0.3.0 writes its zip archive itself and deflates each file in
pieces on every thread, where 0.2 deflated the whole feed on one: on 8
threads Germany converts in 200 s instead of 364 s, Austria's nine
deliveries in 39 s instead of 68 s, Norway in 16 s instead of 29 s. Reading
is as it was.

### How it was measured

`tools/bench/netex_bench.py` on 2 November 2026 over Germany, Norway and
Austria's nine deliveries as `tools/bench/deliveries.json` lists them, on 8
threads, `netex stats`, `netex convert --to gtfs` and `netex convert --to
gtfs --shapes`, one run of each per call; before — netex-cli 0.3.1 — and
after — netex-cli 0.4.0 — called in the order before, after, after, before,
every call one hold of the host's measuring lock. The host as below; the
container was granted 8 CPUs and 16 GiB. The load of the host (one minute)
stood between 1.6 and 36 at the starts of the 132 runs, 12.7 in the median.
Both builds were given the same command lines, `--time-zone Europe/Vienna`
for the Austrian deliveries among them, which 0.3.1 needs.

| Delivery | Step | Before, runs | After, runs | Before | After | Change | Peak memory, before / after |
|---|---|---:|---:|---:|---:|---:|---:|
| Germany | read (`stats`) | 116.0 s, 97.6 s | 103.5 s, 109.2 s | 106.8 s | 106.3 s | −0.4 % | 12.6 / 12.7 GiB |
| Germany | → GTFS | 377.6 s, 350.1 s | 200.6 s, 199.2 s | 363.8 s | 199.9 s | −45.1 % | 12.7 / 12.7 GiB |
| Germany | → GTFS with shapes | 359.1 s, 357.8 s | 209.3 s, 208.2 s | 358.5 s | 208.7 s | −41.8 % | 12.7 / 12.9 GiB |
| Austria, nine deliveries | read | 21.5 s, 21.7 s | 21.8 s, 22.1 s | 21.6 s | 21.9 s | +1.4 % | 1.4 / 1.4 GiB |
| Austria, nine deliveries | → GTFS | 67.5 s, 68.2 s | 39.1 s, 39.3 s | 67.9 s | 39.2 s | −42.3 % | 1.4 / 1.4 GiB |
| Austria, nine deliveries | → GTFS with shapes | 138.7 s, 139.6 s | 74.6 s, 75.0 s | 139.1 s | 74.8 s | −46.2 % | 1.4 / 1.4 GiB |
| Norway | read | 8.3 s, 8.5 s | 8.6 s, 8.6 s | 8.4 s | 8.6 s | +2.2 % | 1.8 / 1.8 GiB |
| Norway | → GTFS | 28.5 s, 28.6 s | 16.2 s, 16.2 s | 28.5 s | 16.2 s | −43.3 % | 1.8 / 1.8 GiB |
| Norway | → GTFS with shapes | 28.6 s, 28.9 s | 32.8 s, 32.6 s | 28.8 s | 32.7 s | +13.7 % | 1.8 / 2.1 GiB |

Austria's times are the sums of its nine deliveries, its memory the
largest one's. Converting reads the delivery too: the feed is written in
what the conversion takes beyond the reading — Germany's in about 94 s
instead of 257 s (most of which, on 16 threads below, was the deflate on one
thread). Norway's conversion with shapes takes longer since it has shapes:
30 396 477 points written where 0.3.1 wrote none, an archive of 328 MB
instead of 70 MB. The feeds have the same rows — but `shapes.txt`, below —
and the same decisions, but for those this release adds and the four
Norwegian operators whose `<Url/>` is empty, now `agency-without-url` (72,
before 68); the trip days held against the publishers' GTFS are what they
were (Germany +3.06 %, Norway equal). The archives are 3 to 7 % smaller
(Germany 666 MB instead of 704 MB, Norway 65 MB instead of 70 MB) at the
same compression level: deflating the feed in pieces of 1 MiB, each primed
with the 32 KiB before it, loses nothing against the one stream of 0.2.

### The same bytes on any number of threads

Converted with `--shapes` on 1, 8 and 16 threads, each feed's archive and
its report are the same bytes (SHA-256, the first 16 digits):

| Delivery | Archive | Report |
|---|---|---|
| Germany | `eaca3f5ec0f2cc61` | `877ba55922b3cb1f` |
| Norway | `b62035e6ac80e9c2` | `93db2b069a9d0230` |
| Austria, railways (ÖBB) | `e3d08a4c3912af75` | `befc256b784ce3c1` |
| Austria, Linz AG | `c8cd10a28801f5ec` | `a566d072314819e8` |
| Austria, Upper Austria | `efe1a6753ce693e4` | `ebf99aad18e302d9` |
| Austria, Salzburg | `9d87a8f5ae396fc8` | `ce1ba4964b557840` |
| Austria, Carinthia | `cbfaa3c6ec69f966` | `12f19e0ffde1d7db` |
| Austria, VOR | `36868f4c32a90530` | `5977054f8d474a3a` |
| Austria, Styria | `bac7b30b353de2a6` | `81f5eae7edffd46b` |
| Austria, Tyrol | `2451ecac46c5c9f0` | `3196029b49e50b05` |
| Austria, Vorarlberg | `7565c8bc0685e523` | `af5c3a99e2d6e332` |

0.3.1 wrote another archive in each run: its two runs of Norway on the same
8 threads came to 69 849 954 and 69 853 937 bytes.

### Shapes

The Nordic deliveries state their link geometries in link sequence
projections, which 0.3.1 did not read:

| Delivery | Trips | With a shape | Points | Links without a geometry |
|---|---:|---:|---:|---:|
| Norway | 374 843 | 359 996 | 30 396 477 | 4 377 |
| Sweden | 438 778 | 388 316 | 19 640 718 | 18 208 |
| Finland | 521 058 | 495 064 | 2 061 520 | — |

Norway's shapes against Entur's GTFS of the same stand: each feed gives
359 996 trips a shape, 359 985 of them the same trips. Of the 26 456 pairs of our shape and Entur's,
4 000 chosen by a hash of the pair (57 218 trips) were measured — each point
of ours against Entur's line and up to 200 points of Entur's against ours,
point to segment: the farthest point lies 0.062 m off, the median of each
trip's farthest 0.000 m. Finland's patterns name one link for the whole way;
2 149 of them name a link between other stop points, and their trips take
the link between their stop points where one is stated (`docs/NORDIC-PROFILE.md`).

Austria's deliveries state their geometry on the link itself, as before, and
name the links of each pattern: a trip now runs over those, else over a link
between its stop points stated in another document. Of the trips with a
shape in both versions, those of the railways' delivery 31 525 of 31 730 have
the same, Styria's 301 917 of 302 472, VOR's 469 317 of 469 322. The others
follow the links their pattern names where the delivery states another link
between two of their calls too: the S3's
`at:obb:ServiceJourney:010S3V-OEBB-1-1-64920:bX:` passes
`at-48-1252-0-5` without stopping, and runs over its pattern's two links
through it (68 and 76 positions), where 0.3.1 took the direct link between
the calls either side (285). 9 110 trips of the railways' delivery and
5 150 of Styria's have a shape now that had none. Germany's 1 438 702 service links state no geometry: with `--shapes`
the report says so and gives every trip `shape-incomplete`, where 0.3.1 said
nothing.

### Time zones

The deliveries that needed `--time-zone` take their zone from the data:

| Delivery | Zone | Decision |
|---|---|---|
| Austria's nine deliveries (frames typed `epip:`) | Europe/Vienna | `time-zone-of-identifiers`, e.g. VOR: 469 994 of 469 994 journeys identified `at:` |
| Belgium, STIB | Europe/Brussels | `time-zone-of-stops`: 3 296 of 3 296 stops with a position in Belgium |
| Belgium, SNCB | Europe/Brussels | 2 721 of 2 950 |
| Belgium, TEC | Europe/Brussels | 45 643 of 46 563 |
| Belgium, De Lijn | Europe/Brussels | 30 213 of 30 596 |
| France, TBM | Europe/Paris | 6 088 of 6 118 |
| France, Naolib | Europe/Paris | 4 097 of 4 100 |
| France, SNCF | Europe/Paris | 3 247 of 3 425 |

Given the same zone with `--time-zone`, the feed is the same bytes (checked
for STIB and Linz AG).

## Whole national deliveries, read and converted to GTFS (0.8.1)

### How it was measured

`tools/bench/netex_bench.py` over `tools/bench/deliveries.json` on 2 November
2026: 34 deliveries of twelve countries, each as its publisher published it
(the address, stand and licence of each are in the list). For each delivery,
on each number of threads, `netex stats` — reading and resolving the
delivery — and `netex convert --to gtfs` into a zip archive, twice each, and
twice more with `--shapes` on the most threads; the tables give the mean of
the two runs. Wall clock and CPU time (user and system) of the process; its
peak resident memory as the kernel's high-water mark, or the highest of the
readings of its resident memory taken every half second where one lay above
it. Before a delivery's first run its files are read once, so that every run
finds them in the page cache alike.

The host: an AMD EPYC 7502P (32 cores, 64 hardware threads, 251 GiB),
shared with other work; the measurements ran in a container granted 16 CPUs
and 64 GiB, the deliveries and the feeds on an NVMe SSD (ext4), Linux 6.8.
netex-cli 0.3.0 with netex-rs 0.8.1 and netex2gtfs 0.2.1, release build —
the code of netex-cli 0.3.1, netex-rs 0.8.2 and netex2gtfs 0.2.2.
Every run held the host's measuring lock, so no other measurement
overlapped it — Germany one hold for each number of threads, the other
deliveries in three holds — but other work ran: the load of the host (one
minute) stood between 3.5 and 42 at the starts of the 348 runs, 27 in the
median. Of the 37 steps taking 20 s or more, the two runs differed by less
than 5 % in 26 and by at most 10.8 %; the shorter steps vary more, by up to
a third for those of a second or less.

### Each delivery

On 16 threads. Read is `netex stats`; → GTFS is `netex convert`, which reads
the delivery too; Peak the larger of the two. Each Dutch operator is read
with the national stop register, one document of 89 MB, counted with it.

| Delivery | Stand | Documents | XML | Read | → GTFS | → GTFS: CPU | Peak | Feed | Trips | Stop times |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Germany (DELFI) | 2026-10-26 | 28 190 | 74.67 GB | 65.4 s | 319.6 s | 14.0 min | 12.83 GiB | 703.8 MB | 4 838 062 | 99 495 018 |
| Austria, railways | 2026-10-08 | 420 | 1.04 GB | 1.2 s | 2.2 s | 11.5 s | 291 MiB | 3.3 MB | 43 957 | 342 403 |
| Austria, Linz AG | 2026-09-24 | 164 | 533.6 MB | 0.6 s | 3.0 s | 7.9 s | 235 MiB | 7.0 MB | 41 945 | 927 405 |
| Austria, Upper Austria (OÖVV) | 2026-10-07 | 384 | 1.60 GB | 3.6 s | 6.1 s | 19.1 s | 728 MiB | 4.9 MB | 44 295 | 814 420 |
| Austria, Salzburg | 2026-10-08 | 213 | 1.04 GB | 1.1 s | 5.1 s | 14.1 s | 591 MiB | 12.1 MB | 62 483 | 1 607 211 |
| Austria, Carinthia | 2026-10-08 | 293 | 923.1 MB | 1.1 s | 3.8 s | 12.0 s | 322 MiB | 7.6 MB | 81 567 | 1 116 164 |
| Austria, Verkehrsverbund Ost-Region | 2026-10-08 | 952 | 5.29 GB | 5.3 s | 25.7 s | 68.6 s | 2.03 GiB | 62.7 MB | 469 994 | 8 972 477 |
| Austria, Styria | 2026-10-08 | 580 | 4.19 GB | 4.4 s | 16.0 s | 49.0 s | 1.26 GiB | 29.5 MB | 307 622 | 4 812 075 |
| Austria, Tyrol | 2026-10-07 | 446 | 1.05 GB | 1.1 s | 4.7 s | 14.4 s | 427 MiB | 9.6 MB | 69 248 | 1 552 546 |
| Austria, Vorarlberg | 2026-10-08 | 313 | 495.0 MB | 0.6 s | 1.8 s | 6.1 s | 225 MiB | 3.6 MB | 24 458 | 465 691 |
| Switzerland | 2026-10-26 (GTFS 2026-10-28) | 521 | 39.33 GB | 58.1 s | 164.4 s | 493.9 s | 16.91 GiB | 337.6 MB | 2 583 297 | 41 602 402 |
| Denmark (Rejseplanen) | 2026-10-30 (GTFS 2026-10-19) | 1 560 | 3.85 GB | 3.1 s | 14.5 s | 43.9 s | 1.12 GiB | 31.8 MB | 196 818 | 4 690 714 |
| Belgium, SNCB | 2026-10-22 (GTFS 2026-10-31) | 2 184 | 7.79 GB | 25.1 s | 32.0 s | 65.6 s | 6.13 GiB | 14.4 MB | 158 370 | 2 427 000 |
| Belgium, De Lijn | 2026-10-31 | 2 023 | 11.87 GB | 5.4 s | 33.8 s | 74.0 s | 2.10 GiB | 84.0 MB | 350 799 | 10 408 080 |
| Belgium, TEC | 2026-10-31 | 849 | 6.64 GB | 3.2 s | 18.7 s | 42.0 s | 1.27 GiB | 38.1 MB | 203 693 | 6 341 138 |
| Belgium, STIB | 2026-10-31 | 89 | 1.18 GB | 0.7 s | 4.2 s | 10.4 s | 552 MiB | 12.8 MB | 76 043 | 1 634 733 |
| Luxembourg | 2026-10-21 | 713 | 651.9 MB | 0.7 s | 3.0 s | 9.0 s | 184 MiB | 7.5 MB | 40 289 | 990 029 |
| Norway (Entur) | 2026-11-01 | 4 818 | 5.71 GB | 8.1 s | 29.3 s | 69.1 s | 1.90 GiB | 69.9 MB | 374 843 | 8 986 827 |
| Sweden (NeTEx Sweden) | 2026-10-30 (GTFS Sweden 3 2026-10-31) | 6 903 | 6.10 GB | 7.6 s | 33.7 s | 82.5 s | 1.82 GiB | 82.0 MB | 438 778 | 9 702 256 |
| Finland (Fintraffic) | 2026-11-01 | 6 098 | 6.31 GB | 8.1 s | 49.3 s | 106.3 s | 1.91 GiB | 120.9 MB | 521 058 | 15 831 600 |
| Netherlands, RET | 2026-10-26 | 2 | 232.7 MB | 1.5 s | 3.6 s | 4.6 s | 285 MiB | 7.4 MB | 43 528 | 888 181 |
| Netherlands, GVB | 2026-10-28 | 2 | 292.9 MB | 1.9 s | 4.6 s | 5.5 s | 349 MiB | 10.2 MB | 70 743 | 1 221 764 |
| Netherlands, Arriva | 2026-10-23 | 2 | 561.6 MB | 4.3 s | 12.2 s | 13.2 s | 608 MiB | 24.7 MB | 142 905 | 3 019 004 |
| Netherlands, Connexxion | 2026-10-31 | 2 | 177.8 MB | 1.4 s | 2.3 s | 2.9 s | 210 MiB | 2.8 MB | 13 902 | 372 461 |
| Netherlands, Qbuzz | 2026-10-23 | 2 | 166.1 MB | 1.3 s | 2.4 s | 3.0 s | 196 MiB | 2.8 MB | 14 290 | 404 599 |
| Netherlands, HTM | 2026-10-19 | 2 | 121.6 MB | 1.3 s | 1.7 s | 1.9 s | 177 MiB | 1.4 MB | 7 147 | 166 214 |
| Czechia, buses | 2026-10-30 | 13 347 | 6.02 GB | 7.8 s | 8.2 s | 65.5 s | 1.62 GiB | 0.0 MB | 0 | 0 |
| Czechia, trams, trolleybuses, cable cars | 2026-10-30 | 909 | 2.78 GB | 2.7 s | 2.5 s | 24.9 s | 519 MiB | 0.0 MB | 0 | 0 |
| Czechia, trains | 2026-10-30 | 13 252 | 1.31 GB | 2.0 s | 1.6 s | 17.5 s | 285 MiB | 0.0 MB | 0 | 0 |
| France, Fluo Grand Est (Bas-Rhin) | 2026-10-30 | 53 | 16.5 MB | 0.1 s | 0.1 s | 0.4 s | 38 MiB | 0.3 MB | 1 771 | 27 800 |
| France, Naolib (Nantes) | 2026-10-30 | 101 | 647.9 MB | 1.0 s | 8.5 s | 16.0 s | 380 MiB | 17.8 MB | 124 908 | 3 031 065 |
| France, TBM (Bordeaux) | 2026-10-31 | 206 | 705.7 MB | 0.7 s | 4.3 s | 10.9 s | 460 MiB | 11.3 MB | 39 749 | 1 579 489 |
| France, Normandy (aggregate) | 2026-10-28 | 686 | 826.9 MB | 1.2 s | 6.3 s | 13.3 s | 458 MiB | 13.6 MB | 85 356 | 1 890 835 |
| France, SNCF | 2026-10-30 | 1 | 647.4 MB | 5.6 s | 7.0 s | 7.0 s | 716 MiB | 4.6 MB | 60 142 | 523 621 |

Every delivery reads whole and converts without error. The three Czech feeds
are empty: no stop place or stop point of the Czech deliveries states a
position, so every stop is left out of the feed (`stop-without-position`) and
every trip with it (`trip-at-unplaced-stops`) — GTFS requires a position for
each stop.

On 8 threads against 16:

| Delivery | Read: 8 / 16 threads | → GTFS: 8 / 16 threads | Peak: 8 / 16 threads |
|---|---:|---:|---:|
| Austria, railways | 1.5 s / 1.2 s | 2.7 s / 2.2 s | 270 MiB / 291 MiB |
| Austria, Linz AG | 0.7 s / 0.6 s | 3.1 s / 3.0 s | 162 MiB / 235 MiB |
| Austria, Upper Austria (OÖVV) | 3.2 s / 3.6 s | 5.9 s / 6.1 s | 602 MiB / 728 MiB |
| Austria, Salzburg | 1.5 s / 1.1 s | 5.5 s / 5.1 s | 377 MiB / 591 MiB |
| Austria, Carinthia | 1.4 s / 1.1 s | 4.3 s / 3.8 s | 230 MiB / 322 MiB |
| Austria, Verkehrsverbund Ost-Region | 7.3 s / 5.3 s | 27.4 s / 25.7 s | 1.41 GiB / 2.03 GiB |
| Austria, Styria | 6.0 s / 4.4 s | 17.3 s / 16.0 s | 1020 MiB / 1.26 GiB |
| Austria, Tyrol | 1.4 s / 1.1 s | 5.0 s / 4.7 s | 283 MiB / 427 MiB |
| Austria, Vorarlberg | 0.7 s / 0.6 s | 1.9 s / 1.8 s | 167 MiB / 225 MiB |
| Switzerland | 75.0 s / 58.1 s | 185.0 s / 164.4 s | 16.59 GiB / 16.91 GiB |
| Denmark (Rejseplanen) | 4.8 s / 3.1 s | 15.9 s / 14.5 s | 806 MiB / 1.12 GiB |
| Belgium, SNCB | 24.4 s / 25.1 s | 32.3 s / 32.0 s | 6.02 GiB / 6.13 GiB |
| Belgium, De Lijn | 7.4 s / 5.4 s | 35.2 s / 33.8 s | 2.07 GiB / 2.10 GiB |
| Belgium, TEC | 4.4 s / 3.2 s | 19.8 s / 18.7 s | 1.25 GiB / 1.27 GiB |
| Belgium, STIB | 0.9 s / 0.7 s | 4.6 s / 4.2 s | 326 MiB / 552 MiB |
| Luxembourg | 1.0 s / 0.7 s | 3.4 s / 3.0 s | 137 MiB / 184 MiB |
| Norway (Entur) | 8.9 s / 8.1 s | 30.6 s / 29.3 s | 1.83 GiB / 1.90 GiB |
| Sweden (NeTEx Sweden) | 9.5 s / 7.6 s | 35.0 s / 33.7 s | 1.70 GiB / 1.82 GiB |
| Finland (Fintraffic) | 9.9 s / 8.1 s | 48.6 s / 49.3 s | 1.84 GiB / 1.91 GiB |
| Netherlands, RET | 1.6 s / 1.5 s | 3.5 s / 3.6 s | 289 MiB / 285 MiB |
| Netherlands, GVB | 1.8 s / 1.9 s | 4.5 s / 4.6 s | 352 MiB / 349 MiB |
| Netherlands, Arriva | 4.1 s / 4.3 s | 12.1 s / 12.2 s | 608 MiB / 608 MiB |
| Netherlands, Connexxion | 1.3 s / 1.4 s | 2.2 s / 2.3 s | 210 MiB / 210 MiB |
| Netherlands, Qbuzz | 1.3 s / 1.3 s | 2.4 s / 2.4 s | 195 MiB / 196 MiB |
| Netherlands, HTM | 1.2 s / 1.3 s | 1.7 s / 1.7 s | 177 MiB / 177 MiB |
| Czechia, buses | 9.8 s / 7.8 s | 10.5 s / 8.2 s | 1.48 GiB / 1.62 GiB |
| Czechia, trams, trolleybuses, cable cars | 3.9 s / 2.7 s | 3.5 s / 2.5 s | 502 MiB / 519 MiB |
| Czechia, trains | 2.4 s / 2.0 s | 2.7 s / 1.6 s | 243 MiB / 285 MiB |
| France, Fluo Grand Est (Bas-Rhin) | 0.1 s / 0.1 s | 0.1 s / 0.1 s | 38 MiB / 38 MiB |
| France, Naolib (Nantes) | 1.5 s / 1.0 s | 9.2 s / 8.5 s | 256 MiB / 380 MiB |
| France, TBM (Bordeaux) | 1.1 s / 0.7 s | 4.7 s / 4.3 s | 302 MiB / 460 MiB |
| France, Normandy (aggregate) | 1.5 s / 1.2 s | 6.8 s / 6.3 s | 393 MiB / 458 MiB |
| France, SNCF | 5.6 s / 5.6 s | 7.3 s / 7.0 s | 716 MiB / 716 MiB |

The rows written to each file of each feed:

| Delivery | agency | attributions | calendar | calendar_dates | frequencies | levels | routes | stop_times | stops | transfers | trips |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Germany (DELFI) | 1 361 | 1 | 17 200 | 94 894 | 0 | 0 | 28 190 | 99 495 018 | 628 172 | 401 356 | 4 838 062 |
| Austria, railways | 13 | 1 | 3 233 | 14 555 | 0 | 0 | 419 | 342 403 | 5 208 | 12 | 43 957 |
| Austria, Linz AG | 1 | 1 | 253 | 3 266 | 0 | 0 | 163 | 927 405 | 1 358 | 0 | 41 945 |
| Austria, Upper Austria (OÖVV) | 14 | 1 | 671 | 6 370 | 0 | 0 | 383 | 814 420 | 13 376 | 58 | 44 295 |
| Austria, Salzburg | 20 | 1 | 696 | 10 303 | 0 | 0 | 183 | 1 607 211 | 7 199 | 24 | 62 483 |
| Austria, Carinthia | 25 | 1 | 657 | 2 914 | 0 | 0 | 249 | 1 116 164 | 7 992 | 24 | 81 567 |
| Austria, Verkehrsverbund Ost-Region | 27 | 1 | 1 553 | 24 774 | 0 | 0 | 929 | 8 972 477 | 31 270 | 352 | 469 994 |
| Austria, Styria | 64 | 1 | 744 | 7 491 | 0 | 0 | 571 | 4 812 075 | 20 959 | 158 | 307 622 |
| Austria, Tyrol | 44 | 1 | 1 499 | 20 048 | 0 | 0 | 442 | 1 552 546 | 10 424 | 106 | 69 248 |
| Austria, Vorarlberg | 27 | 1 | 679 | 20 534 | 0 | 0 | 312 | 465 691 | 5 997 | 30 | 24 458 |
| Switzerland | 368 | 1 | 53 020 | 1 103 273 | 578 | 0 | 4 007 | 41 602 402 | 91 414 | 7 287 | 2 583 297 |
| Denmark (Rejseplanen) | 56 | 1 | 1 800 | 5 636 | 0 | 0 | 1 560 | 4 690 714 | 59 103 | 62 588 | 196 818 |
| Belgium, SNCB | 0 | 1 | 12 723 | 286 912 | 0 | 0 | 2 182 | 2 427 000 | 2 946 | 0 | 158 370 |
| Belgium, De Lijn | 0 | 1 | 1 222 | 2 984 | 0 | 0 | 2 021 | 10 408 080 | 25 505 | 0 | 350 799 |
| Belgium, TEC | 0 | 1 | 178 | 189 | 0 | 0 | 847 | 6 341 138 | 46 563 | 0 | 203 693 |
| Belgium, STIB | 0 | 1 | 42 | 14 | 0 | 0 | 87 | 1 634 733 | 3 296 | 0 | 76 043 |
| Luxembourg | 6 | 1 | 258 | 656 | 0 | 0 | 713 | 990 029 | 8 038 | 7 043 | 40 289 |
| Norway (Entur) | 252 | 47 | 3 938 | 24 760 | 0 | 0 | 4 049 | 8 986 827 | 140 323 | 72 702 | 374 843 |
| Sweden (NeTEx Sweden) | 236 | 1 | 6 009 | 61 822 | 0 | 0 | 6 901 | 9 702 256 | 142 829 | 68 114 | 438 778 |
| Finland (Fintraffic) | 116 | 1 | 844 | 237 | 0 | 0 | 2 429 | 15 831 600 | 88 820 | 0 | 521 058 |
| Netherlands, RET | 1 | 3 | 52 | 33 | 0 | 0 | 96 | 888 181 | 2 965 | 0 | 43 528 |
| Netherlands, GVB | 1 | 3 | 90 | 39 | 0 | 0 | 76 | 1 221 764 | 1 978 | 0 | 70 743 |
| Netherlands, Arriva | 1 | 4 | 65 | 661 | 0 | 0 | 594 | 3 019 004 | 16 038 | 0 | 142 905 |
| Netherlands, Connexxion | 3 | 4 | 55 | 11 | 0 | 0 | 112 | 372 461 | 4 141 | 0 | 13 902 |
| Netherlands, Qbuzz | 1 | 3 | 14 | 0 | 0 | 0 | 119 | 404 599 | 5 957 | 0 | 14 290 |
| Netherlands, HTM | 1 | 4 | 4 | 0 | 0 | 0 | 22 | 166 214 | 723 | 0 | 7 147 |
| Czechia, buses | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| Czechia, trams, trolleybuses, cable cars | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| Czechia, trains | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| France, Fluo Grand Est (Bas-Rhin) | 10 | 1 | 24 | 431 | 0 | 0 | 50 | 27 800 | 1 555 | 0 | 1 771 |
| France, Naolib (Nantes) | 1 | 1 | 62 | 90 | 0 | 0 | 98 | 3 031 065 | 3 631 | 0 | 124 908 |
| France, TBM (Bordeaux) | 1 | 1 | 45 | 172 | 0 | 0 | 128 | 1 579 489 | 6 118 | 0 | 39 749 |
| France, Normandy (aggregate) | 33 | 32 | 1 166 | 6 077 | 0 | 0 | 590 | 1 890 835 | 25 028 | 70 | 85 356 |
| France, SNCF | 7 | 1 | 7 546 | 43 565 | 0 | 0 | 740 | 523 621 | 3 425 | 0 | 60 142 |

### Germany over the threads

| Threads | Read | Read: CPU | → GTFS | → GTFS: CPU | Peak |
|---:|---:|---:|---:|---:|---:|
| 1 | 567.5 s | 567.1 s | 13.5 min | 13.5 min | 11.07 GiB |
| 4 | 164.2 s | 568.9 s | 420.6 s | 13.8 min | 12.42 GiB |
| 8 | 97.7 s | 566.9 s | 350.3 s | 13.6 min | 12.66 GiB |
| 16 | 65.4 s | 572.0 s | 319.6 s | 14.0 min | 12.83 GiB |

Reading scales with the threads: 9.5 minutes on one, 65 s on sixteen, the CPU
time the same on each (9.5 minutes). Converting takes about 250 s more on any
number of threads (245 s on one, 254 s on sixteen): the feed is written on
one thread — 4.84 million trips with 99.5 million stop times, 9.2 GB of text
deflated into a 704 MB archive. Written into a directory instead of a zip
archive, the same conversion takes 155 s on sixteen threads (154.8 and
155.3 s, two runs under the same lock, peak 12.8 GiB alike): of the 254 s
spent writing, 165 s are deflating the archive, file after file on the one
thread. Deflating the files of the archive side by side, or writing the
directory and archiving it apart, would save most of that; neither is done
here.

### Where the memory goes

The resident memory of the conversion over time (Germany, 16 threads, first
run): it rises while the documents are read, to 10.3 GiB at 40 s; peaks at
12.8 GiB at 57 s, where the index builders of the threads are merged into one
index; falls to 8.0 GiB once they are; and stays there while the feed is
written, until the process ends at 319 s. On one thread the read climbs
steadily to the peak of 11.0 GiB at 548 s and the feed is written from
6.0 GiB; on four and eight threads the peak is 12.4 and 12.7 GiB, the writing
7.4 and 7.6 GiB. So:

- **The conversion needs no more memory than reading the delivery.** The
  peak of `netex convert` is that of `netex stats` on the same threads, within
  0.03 GiB, for every delivery.
- **The peak falls at the end of the read**, where the index builders of the
  threads are merged into one; more threads, more builders (Germany 11.1 GiB
  on one, 12.8 GiB on sixteen).
- **Writing holds the index** and what the feed needs beside it: 6 to 8 GiB
  for Germany.

Germany converts in under 13 GiB, which a machine with 16 GiB holds.
**Switzerland** needs more: 16.6 GiB on 8 threads, 16.9 GiB on 16, 13.6 GiB
while the feed is written — its index of 2.6 million journeys, with their
calls and availability conditions, is larger than Germany's of 4.8 million,
and its timetable documents run to 717 MB, each inflated whole by the thread
reading it. On fewer
threads it needs less: 12.4 GiB on one, reading (`netex stats`) as
converting (`netex convert`, 8.0 minutes), and 16.2 GiB reading on four —
measured for the memory, outside the lock. On a machine with 16 GiB,
Switzerland converts on one thread.

A leaner conversion without loss would have to keep the index's merge from
holding the builders and the merged index at once, or read the delivery
document by document into the feed: a German line document states its own
stops, but stops and transfers are merged across documents, so the feed of
one document is not a part of the feed of the delivery. That is a change of
the index, not made here.

### Shapes

`--shapes` writes the geometry a service link states itself, as a
`gml:LineString` on the link:

| Delivery | → GTFS with shapes | Peak | Feed | Shape points | Trips without a shape |
|---|---:|---:|---:|---:|---:|
| Austria, railways | 22.5 s | 573 MiB | 103.8 MB | 10 937 713 | 12 227 |
| Austria, Linz AG | 3.8 s | 238 MiB | 10.0 MB | 471 186 | 577 |
| Austria, Upper Austria (OÖVV) | 16.1 s | 723 MiB | 51.4 MB | 5 412 510 | 37 |
| Austria, Salzburg | 9.8 s | 587 MiB | 33.0 MB | 2 504 722 | 0 |
| Austria, Carinthia | 7.7 s | 326 MiB | 26.1 MB | 2 370 941 | 0 |
| Austria, Verkehrsverbund Ost-Region | 43.7 s | 2.04 GiB | 142.1 MB | 9 585 055 | 672 |
| Austria, Styria | 24.6 s | 1.24 GiB | 68.5 MB | 4 750 193 | 5 150 |
| Austria, Tyrol | 14.0 s | 429 MiB | 55.5 MB | 5 456 531 | 0 |
| Austria, Vorarlberg | 5.6 s | 227 MiB | 22.5 MB | 2 283 945 | 0 |
| Denmark (Rejseplanen) | 21.3 s | 1.13 GiB | 58.8 MB | 4 062 325 | 3 727 |

The other deliveries give no shapes. Norway's, Sweden's and Finland's
service links state their geometry in their projections
(`projections/LinkSequenceProjection/gis:LineString`, 13 377 of Norway's
13 403 links), in the shared documents of each codespace rather than the
line documents; netex2gtfs does not read it there, and its report does not
say so. The service links of Germany (1.44 million), Luxembourg and the
Czech buses and city transport state no geometry; Switzerland's, Belgium's,
the Dutch and the French deliveries state no service links (TBM writes its
geometry in general frames of link projections, `docs/FR-PROFILE.md`).
Converting with `--shapes` costs about what converting without does where no
shape is written, and more where many are: Austria's railways 22.5 s against
2.2 s for 10.9 million shape points, Denmark 21.3 s against 14.5 s.

### Each feed against its publisher's GTFS

Where the publisher writes a GTFS feed of its own from the same timetable, the
feed `netex convert` wrote is held against it (`tools/bench/gtfs_days.py`):
the trips running in the week of 2 to 8 November 2026 and their trip days — a
trip once for each day it runs —, counted from `trips.txt`, `calendar.txt`
and `calendar_dates.txt` of both alike. A trip of `frequencies.txt` counts
once a day in either feed.

| Delivery | Trips: written | Trips: publisher | Trip days: written | Trip days: publisher | Difference |
|---|---:|---:|---:|---:|---:|
| Germany (DELFI) | 1 626 088 | 1 554 798 | 5 296 140 | 5 138 866 | +157 274 (+3.06 %) |
| Switzerland | 617 293 | 618 549 | 1 234 031 | 1 236 382 | −2 351 (−0.19 %) |
| Denmark (Rejseplanen) | 99 086 | 97 218 | 268 166 | 267 815 | +351 (+0.13 %) |
| Belgium, SNCB | 8 172 | 8 737 | 26 752 | 26 978 | −226 (−0.84 %) |
| Belgium, De Lijn | 74 068 | 75 995 | 180 058 | 180 062 | −4 (−0.00 %) |
| Belgium, TEC | 76 052 | 76 052 | 117 466 | 117 466 | 0 (+0.00 %) |
| Belgium, STIB | 64 533 | 64 533 | 115 548 | 115 548 | 0 (+0.00 %) |
| Norway (Entur) | 181 921 | 181 921 | 441 779 | 441 779 | 0 (+0.00 %) |
| Sweden (NeTEx Sweden) | 251 598 | 251 598 | 804 508 | 804 156 | +352 (+0.04 %) |
| Finland (Fintraffic) | 225 560 | 216 873 | 466 816 | 367 705 | +99 111 (+26.95 %) |
| France, Fluo Grand Est (Bas-Rhin) | 1 513 | 1 513 | 5 529 | 5 529 | 0 (+0.00 %) |
| France, Naolib (Nantes) | 36 127 | 36 127 | 54 037 | 54 037 | 0 (+0.00 %) |
| France, TBM (Bordeaux) | 19 904 | 19 904 | 43 113 | 43 113 | 0 (+0.00 %) |

- **Equal**: Norway, TEC, STIB, Fluo Grand Est, Naolib and TBM — the same
  number of trips on the same number of days. Norway's, TEC's and STIB's
  feeds were paired with their publishers' trip for trip by identifier on
  earlier stands (`docs/NORDIC-PROFILE.md`, `docs/EPIP-NEIGHBOURS.md`), the
  French networks' in `docs/FR-PROFILE.md`.
- **Germany, +3.06 %.** DELFI's GTFS feed is an export of its own, not a
  conversion of the NeTEx. Matched run by run — line name, first departure,
  last arrival and number of calls, on each day — the written feed runs
  171 024 trip days the export does not, and the export 13 750 the feed does
  not. 129 503 of the 171 024 lie on 616 lines of 74 operators none of whose
  runs the export carries (55 lines of Energie Waldeck-Frankenberg with
  60 507, 77 of KVG Stade with 14 364, …); the NeTEx states them with their
  times and days, and the conversion keeps them. The rest lie on 394 lines
  the export carries in part. Apart from these the run-days are alike, but
  not the trips: the export writes runs of one line with the same times as
  one trip running on all their days — 2.94 million trips against the
  written feed's 4.84 million, one for each run. Journeys stated twice on the
  same day are not the cause: they are 0.19 % of the written feed's trip days
  and 0.18 % of the export's.
- **Finland, +26.95 %**, a defect of the delivery: 946 day types — 822 of
  them Turku's (FÖLI), the rest in eleven other codespaces — have a property
  of day naming no day of the week, which by NeTEx runs their journeys on
  every day of their periods; Fintraffic's GTFS has the weekdays. FÖLI alone
  runs 85 043 trip days in the written feed against 26 059 in Fintraffic's.
  The regional buses of Oulu, Lahti and Kuopio run 2.8 to 3.8 times their
  trip days in Fintraffic's feed too; that was not traced further. The other
  way, 1 385 references to quays the delivery's stop register does not hold
  (railway and Helsinki metro platforms) leave 434 508 calls without a place
  and 18 314 trips with fewer than two calls, which the feed leaves out.
  `netex validate` reports both (`property-of-day-without-days`,
  `stop-reference-to-nothing`).
- **Sweden +0.04 %, Denmark +0.13 %, Switzerland −0.19 %, SNCB −0.84 %**:
  feeds and deliveries of different days (Sweden one day apart, Switzerland
  two, SNCB nine, Denmark eleven) — the same number of trips in Sweden,
  1 868 trips more in Denmark's newer NeTEx. Switzerland's feeds also state
  runs at a headway otherwise: 578 rows of `frequencies.txt` in the written
  feed, 1 905 in the publisher's. None of the four was traced further.
- **De Lijn, −4 trip days**: NeTEx and GTFS are not of one export; 1 927
  trips fewer in the week run on 4 trip days fewer (`docs/EPIP-NEIGHBOURS.md`).

For Austria, Luxembourg, Czechia and Normandy no feed of the same stand was
at hand; the Netherlands publish one national feed for all operators, against
which `docs/NL-BISON-PROFILE.md` holds the six operators trip for trip.
SNCF's delivery, under the ODbL, was measured, not compared.

### The GTFS validator

The validator of MobilityData, 8.0.1, on the feeds of Germany and Norway and
on their publishers', its rules of time taken on 2 November 2026:

| Feed | Errors | Warnings | Infos | Errors by code |
|---|---:|---:|---:|---|
| Germany, written by `netex convert` | 1 361 | 539 539 | 2 055 | `missing_required_field` 1 361 |
| Germany, the publisher's | 22 027 | 724 415 | 102 181 | `number_out_of_range` 21 923, `block_trips_with_overlapping_stop_times` 92, `location_with_unexpected_stop_time` 12 |
| Norway, written by `netex convert` | 77 | 24 551 | 1 129 | `missing_required_field` 72, `transfer_with_invalid_trip_and_stop` 4, `invalid_url` 1 |
| Norway, the publisher's | 23 | 59 013 | 431 | `duplicate_key` 13, `decreasing_or_equal_stop_time_distance` 6, `transfer_with_invalid_trip_and_stop` 4 |

All but five of the errors in the feeds written are agencies without a web
address (`agency_url`), which GTFS requires: `netex convert` writes the
publisher's where the delivery states none, and the measurements gave no
publisher. Written with one (`--publisher-name`, `--publisher-url`), outside
the measurements, Germany's feed has **no error** (539 584 warnings, most of
them `fast_travel_between_consecutive_stops`, as in DELFI's own feed) and
Norway's has nine: the four transfers between trips at stops they do not
call at that Entur's own feed has too; the web address
`https://www.boreal.no//` as the delivery writes it; and four taxi operators
whose delivery writes an empty `<Url/>`, which netex2gtfs takes for an
address and writes empty, without a decision.

## Reading the German delivery fast (0.2, 0.3): what was measured, what was changed

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

### Before: the curve knee and its cause

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

### After: what changed

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

### After: the curve

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

### The lean path

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

### Where the time goes now

Full model, 8 threads (`flamegraph-after-full-8-threads.svg`): typed reading
78 % of the samples, of it quick-xml's `read_event` 12 %; inflating into the
worker's buffer 6 %; freeing the documents 3 %; merging 2 %; resolving 1 %.
Opening the archive, which took a fifth of the samples before, is gone from
the profile. (pprof samples by `SIGPROF`; under many threads the signals
coalesce, so the shares hold and the absolute counts do not.)

### Guards

`tests/guards.rs` counts what reading the 1.1 MB fixture allocates (exact, so
the thresholds sit a fifth above today's figures) and times it (best of seven,
floors at a third of today's throughput, release builds only). `cargo bench`
has `read` (full model), `read_lean` (routing profile), `write`, `index`, and
`dataset` (the five fixtures indexed full and lean), one thread each.

### 0.3.0: what answering the questions costs

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
