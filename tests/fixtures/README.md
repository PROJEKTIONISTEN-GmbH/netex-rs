# Test fixtures

Five kinds of test data:

| Directory | What | Licence | In the crate's package |
|---|---|---|---|
| `own/` | small deliveries written for these tests (EPIP, IT-ITH1, Austrian-style, Swiss-style, NeTEx France, Nordic NeTEx Profile with its stop register; see its `README.md`) | MIT OR Apache-2.0, as the crate | yes |
| `fragments/` | sets of frames and entities standing alone written for these tests, each pinning one kind of thing a delivery states (see its `README.md`) | MIT OR Apache-2.0, as the crate | yes |
| `delfi/` | five line files of the German national NeTEx delivery | CC BY 4.0, DELFI e.V. | yes |
| `crs/` | reference points of the feature `crs`, generated from two stop registers and PROJ | see below | yes |
| `netex-1.3/`, `netex-2.0/` | the official schemas and examples of both releases | GPL-3.0 | no |

The official schemas and examples are left out of the package (`exclude` in
`Cargo.toml`); the tests that need them read them where they are, as in a clone
of the repository, and are skipped with a note where they are not.

## `netex-1.3/` and `netex-2.0/`

Everything under `netex-1.3/` and `netex-2.0/` is a verbatim copy of published CEN
NeTEx material; nothing has been edited. The point of the harness is that the crate
has to match these documents and schemas, not the other way round.

- **Source**: the NeTEx XML Schema repository,
  <https://github.com/TransmodelEcosystem/NeTEx>
- **Licence**: GPL-3.0 (the repository's `LICENSE`, copied alongside). They are
  test data of this repository and the crate's package does not carry them. What
  the crate takes from them are facts of the standard — the names of elements,
  attributes and enumeration tokens and which release declares them — listed in
  `src/version/table.rs` and `docs/VERSIONS.md` by `tools/schema_table.py`, and
  checked against them by the tests.

| Directory | Branch and commit | Schemas | Example documents |
|---|---|---|---|
| `netex-1.3/` | `v1.3` @ `4f42794` ("Fix VDV examples (#957)") | 409 `.xsd` under `xsd/`, root `xsd/NeTEx_publication.xsd` (schema version `1.1`) | 265 `.xml` under `examples/` |
| `netex-2.0/` | `v2.0` @ `8056d5a` ("Should fail" examples framework (#1074), 18 September 2026) | 450 `.xsd` under `xsd/`, root `xsd/NeTEx_publication.xsd` (schema version `2.0`) | 290 `.xml` under `examples/`, including `examples/should-fail/` |

Each file of both directories was compared with the file of the same path at the
commit above (by its git blob hash; on 2026-10-28): all 722 files of `netex-1.3/`
and all 791 of `netex-2.0/` are byte for byte those of the NeTEx repository, the
`LICENSE` at its root included. The commits in full:
`4f42794047ec9944fd38e4497f3af142a33462c8` and
`8056d5a30b131a3303d5c8b198cadc3a3570b083`.

The example documents, by the directory they stand in, each the directory of the
same path in the NeTEx repository — files, and of them XML documents, in
parentheses; the others are the images, spreadsheets, PDFs and notes the
repository keeps beside its examples, copied with them:

| Directory | `netex-1.3/` | `netex-2.0/` |
|---|---:|---:|
| `examples/functions/bookingArrangements/` | — | 1 (1) |
| `examples/functions/calendar/` | 3 (3) | 3 (3) |
| `examples/functions/deckPlans/` | — | 4 (4) |
| `examples/functions/deprecation/` | 1 (1) | 1 (1) |
| `examples/functions/fares/` | 17 (17) | 17 (17) |
| `examples/functions/grouping/` | 1 (1) | 1 (1) |
| `examples/functions/line/` | — | 2 (2) |
| `examples/functions/newModes/` | 5 (5) | 5 (5) |
| `examples/functions/patterns/` | 2 (1) | 3 (2) |
| `examples/functions/pointOfInterest/` | 1 (1) | 1 (1) |
| `examples/functions/privateCodes/` | — | 1 (1) |
| `examples/functions/relations/` | — | 2 (2) |
| `examples/functions/simpleNetwork/` | 5 (2) | 6 (3) |
| `examples/functions/site/` | 21 (9) | 21 (9) |
| `examples/functions/stopPlace/` | 13 (7) | 13 (7) |
| `examples/functions/timetable/` | 24 (23) | 28 (27) |
| `examples/functions/validityCondition/` | 1 (1) | 1 (1) |
| `examples/functions/variant/` | 1 (1) | 1 (1) |
| `examples/functions/vehicle/` | — | 1 (1) |
| `examples/functions/vehicleSchedule/` | 1 (1) | 1 (1) |
| `examples/functions/versioning/` | 10 (10) | 10 (10) |
| `examples/should-fail/` | — | 4 (3) |
| `examples/standards/efp/` | — | 8 (8) |
| `examples/standards/epiap/` | 6 (4) | 6 (4) |
| `examples/standards/epip/` | 1 (1) | 1 (1) |
| `examples/standards/era_uic/` | 15 (14) | 14 (14) |
| `examples/standards/fxc/` | 21 (21) | 21 (21) |
| `examples/standards/gbfs/` | 1 (1) | 1 (1) |
| `examples/standards/gtfs/` | 31 (20) | 31 (20) |
| `examples/standards/neptune/` | 4 (3) | 4 (3) |
| `examples/standards/noptis/` | 3 (3) | 3 (3) |
| `examples/standards/norway/` | 36 (35) | 36 (35) |
| `examples/standards/nta/` | 13 (13) | 13 (12) |
| `examples/standards/tap_tsi/` | 36 (33) | 37 (33) |
| `examples/standards/tfl/` | 1 (0) | 1 (0) |
| `examples/standards/txc/` | 13 (13) | 13 (11) |
| `examples/standards/vdv452/` | 6 (3) | 6 (3) |
| `examples/ws_siri_request/siri_generic/` | 11 (11) | 11 (11) |
| `examples/ws_siri_request/siri_netex/` | 7 (7) | 7 (7) |

The 1.x directory also holds `examples/index.htm`, the repository's index page of
its examples.

The `xsd/wsdl/` and `xsd/wsdl_model/` directories are omitted from both: they
describe SOAP bindings rather than the document grammar, and nothing here validates
against them. Everything else under `xsd/` — the SIRI, IFOPT and GML schemas the
NeTEx schemas import — is present, because the root schema includes it.

The example directories hold, besides NeTEx documents, a few SIRI request envelopes
(`ws_siri_request/`), one TransXChange and one NaPTAN document (`standards/txc/`);
the harness recognises them by their root namespace and lists them as skipped.

## `delfi/`

Five line files from the German national access point's NeTEx delivery, kept as
typical small inputs (the whole delivery is 28 176 such files and is read from
outside the repository by the corpus test).

- **Source**: DELFI e.V., "Deutschlandweite Sollfahrplandaten (NeTEx)",
  <https://www.opendata-oepnv.de/> — delivery `20260921_fahrplaene_gesamtdeutschland.zip`
  (files dated 2026-09-19 inside).
- **Licence**: Creative Commons Attribution 4.0 International (CC BY 4.0),
  <https://creativecommons.org/licenses/by/4.0/>. Attribution: © DELFI e.V.
  The five files are taken from the delivery unmodified.
- **Profile**: `NX-PI-01_DE_NAP_LINE`, one `PublicationDelivery version="ntx:1.1"`
  per line, a `CompositeFrame` holding resource, calendar, service, site and
  timetable frames.

| File | Why it is here |
|---|---|
| `NX-PI-01_DE_NAP_LINE_vgm_d-VKU_50-12816_20260919.xml` | the smallest file of the delivery (16 KB): one stop, one journey |
| `NX-PI-01_DE_NAP_LINE_DBDB-54____-PPN_20260919.xml` | a small Deutsche Bahn file (19 KB) |
| `NX-PI-01_DE_NAP_LINE_WEB-WEB_HB-511699_20260919.xml` | quays, levels, site connections, destination displays (35 KB) |
| `NX-PI-01_DE_NAP_LINE_DBDB-8015FR-RE19_20260919.xml` | journey interchanges, stop place entrances, transport submodes (270 KB) |
| `NX-PI-01_DE_NAP_LINE_DBDB-87____-TER_20260919.xml` | the line the RE19 file's interchanges lead to; together the two resolve across files (1.1 MB) |

## `crs/`

`reference.tsv` holds the positions the feature `crs` is checked against: per
reference system, a position as written in it and the same position in WGS 84,
with where it comes from. It is generated by `tools/crs_corrections.py`, which
also writes the corrections in `src/crs/corrections.rs`:

- **Switzerland**: service points of the Swiss service point register
  (opentransportdata.swiss), stated by the publisher in LV95 and in WGS 84; LV03
  through swisstopo's grid CHENyx06 (© swisstopo, free use).
- **Austria**: stops of the Austrian stop register (data.gv.at, CC BY 4.0), in
  WGS 84; MGI Lambert and Gauss-Krüger M31 through the BEV's grid AT_GIS_GRID
  (© BEV, CC BY 4.0).
- **The Netherlands, France, the UTM zones**: a lattice over the country,
  projected with PROJ (RD New through NSGI's RDNAPTRANS2018).

No file is copied: the rows are positions turned into each system with PROJ.

## `own/` and `fragments/`

Written for these tests, with invented places, lines, operators and times; their
own `README.md` lists each file and what it pins.
