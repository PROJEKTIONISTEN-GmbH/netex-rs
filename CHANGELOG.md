# Changelog

## 0.12.1 — unreleased

netex2gtfs writes valid GTFS where it did not: the last run of every headway
group, no identifier twice, one time zone for all agencies, times at every
trip's ends, no shape without positions, no required name or web address
left empty where the delivery states one. netex-rs's code is that of 0.12.0.

Of the 21 national deliveries at hand, converted before and after, the files
of 18 feeds are the same bytes (the archive holds `stop_times.txt` before
`trips.txt` now); Switzerland's `frequencies.txt` ends each of its 582 groups
a second after its last departure, so that its last run runs;
Denmark's writes the agency of Greenland's buses in the feed's zone, where
its own (America/Nuuk) made a feed of two zones; Sweden's names an operator
stated without a name by its identifier.

### netex2gtfs 0.7.0

Built on netex-rs 0.12.1. A new minor version, as a feed written into a
directory now removes what an earlier one left there, and the feed's agencies
share one time zone.

- `frequencies.txt` ends a second after a headway group's last departure:
  GTFS runs a frequency up to, not including, its end, so that the last run
  of every group was lost — Seeweg's 06:00 to 07:00 every 20 minutes ran at
  06:00, 06:20 and 06:40. Where the feed leaves out a template journey's
  first call, its groups start as much later as its first call written is.
- Identifiers made unique (`stop-made-unique`, `trip-made-unique`,
  `route-made-unique`) take the next number no identifier has taken: `a#2`,
  `a`, `a` gave `a#2` twice; a route numbered `L#2` took the identifier of a
  line `L#2`. Routes are found by their definition in one table, not by a
  search of all routes for each line.
- One time zone for all agencies, as GTFS requires: the first a document
  names by its name in the time zone database, in the order of the
  documents' names. An organisation whose document names another is
  written in the feed's (`agency-time-zone-unified`); a document naming an
  abbreviation of an offset
  (`CET`, `CEST`, `BST`, as the standard's examples write it) names none
  (`time-zone-not-a-name`), where it was written as the agencies' zone.
  The trips of an agency written in the feed's zone keep their times as
  stated: where the zones' offsets differ — America/Nuuk beside
  Europe/Copenhagen, three hours — they run at other instants than the
  delivery's, which the report says.
- A trip begins and ends at a call with a time: a call left out at an end
  (`call-without-place`, `stop-without-position`) left one without
  (`trip-untimed-end-cut`). A trip's course of fewer than two positions gives
  no shape (`shape-without-two-positions`), where `shape_id` named an empty
  one.
- An organisation without a web address takes the feed publisher's — the
  one given, else the delivery's data source with one, as `feed_info.txt`
  names it —, and a line naming no organisation takes that publisher as its
  agency; an organisation, line or stop without a name is named by its
  identifier (`agency-named-by-its-identifier`,
  `route-named-by-its-identifier`, `stop-named-by-its-identifier`).
- `output::Directory`: the files of the GTFS reference a conversion does not
  write are removed when it finishes; a `shapes.txt` or `feed_info.txt` of an
  earlier run stayed beside the new feed.
- A destination display holds across a call its pattern does not name; it
  jumped to the pattern's last. A parent site without a position is no
  station (`parent-site-without-position`); it was left out without a word.
  Colours are read by their local name, in whatever prefix. The feed's
  version is the latest publication timestamp in time, not as text. A
  trolleybus is route type 11. Iceland and Cyprus are in the table of
  countries, as the README promised. Once a document does not read, no
  thread reads another — the source still loads each —, and of the
  documents found failing the one named first is the error.
- `stop_times.txt` is written before `trips.txt`, so that each trip's calls
  and displays are found once.

### netex-cli 0.8.0

Built on netex-rs 0.12.1 and netex2gtfs 0.7.0: `netex convert` writes the
feed as netex2gtfs 0.7.0 does; into a directory, it removes what a feed
written there before left. `--time-zone` names the feed's one zone.

## 0.12.0 — unreleased

The index and the merge answer alike whatever order the documents come in
and whatever form the schema allows a value: no journey kept once points at
another, no name stated in NeTEx 2.0's forms is empty, no day offset or
duration overflows, no date, bound or day bit is read a day off. Breaking,
and so the next minor version: the calendar functions take every validity of
an entity, and the interfaces named below change. Every national delivery at
hand — 21, of ten countries — reads to the same statistics as with 0.11.0,
and the feeds of seven — Austria's VOR, Belgium's railways, Czechia's
trains, Sweden, Norway, Switzerland, Germany — are the same bytes.

- **index**: of the versions valid on the day an index is built for
  (`IndexBuilder::at`), the highest answers; the one added or merged last
  did. Versions are ordered as one order of every two — none, then those
  in digits by their number (`1` and `01` by their text), then the others
  by their text: `9` < `10` < `2.0` —, where `9` and `2.0` compared as text
  and `10` and `9` as numbers, and the highest of three hung on the order
  they were met in. Each identifier's versions are listed once, highest
  first: a reference naming `any` no longer searches the whole table.
- **index**: a stop assignment and the end of a walk (`SiteConnection`)
  naming a quay that does not resolve take the stop place they name beside
  it — DELFI names both —; the quay stays a reference to nothing. The stop
  point had no stop and the walk was gone. A level reference resolves as
  every reference: one to nothing is listed by `Index::unresolved`, where
  it said nothing, and one past its version is a finding of
  `resolved-ignoring-version`. Of several stops of one DHID and of one
  identifier and version, the document named first answers, and walks
  alike but for their direction are listed in one order, where both hung
  on the order the documents came in; so walks stated twice alike lie side
  by side and are kept once — Germany's index holds 4 930 fewer, its feed
  the same bytes, netex2gtfs's count of `walk-stated-twice` 11 103 where it
  was 16 033.
- **index**: a day offset is counted without overflow; a time before the
  midnight beginning its operating day — a negative offset — or 2^32
  seconds or more after it is no time, and a finding about its journey, a
  new concern (`Concern::TimeOutOfRange`, `time-out-of-range`): an offset of
  106 751 991 167 301 days panicked in a debug build and gave a wrong time in
  a release build. The 31 negative offsets of Czechia's trains, in five
  journeys, were no time before as well; the five are a finding now.
- **index**, the validity of an entity (what `at` resolves by): every
  `ValidBetween` it states counts, within its frames' validity — it
  replaced it —, and a frame's includes what its `validityConditions` state
  (`stream::FrameHeader::valid_between` holds them too). Of the 2 498 738
  `ValidBetween` of the deliveries at hand none follows another; 253 stand
  in a frame's validity conditions, all of them Norway's.
- **calendar** (breaking): `Span::all_of` reads every `ValidBetween` of an
  entity, where `Span::of` read the first; `governed` takes each version as
  its validities (`&[Vec<Span>]`), so that a line valid in two seasons
  governs the second too; a day type's validity is each of them. Bounds —
  of periods, conditions and validities alike — name the next day only as a
  whole hour in UTC from noon on (`bound_day`): the end of a day written
  with a zone (`T23:59:59Z`, `T23:59:59+01:00`) and a time in a zone of its
  own (`T23:00:00+01:00`) are the day of their date, where a period ending
  so ran a day longer; conditions and validities read the date alone, a day
  before a period of the same bound. Belgium's 123 630 bounds at UTC
  midnights read as before, and so do the only two others written with a
  zone from noon on, Norwegian conditions ending `2030-01-10T14:45:00.000Z`,
  on the day of their date.
- **calendar**, day bits: white space among them is no bit; a bit other
  than `0` or `1` is no day (`PeriodIssue::BitOtherThanZeroOrOne`, a new
  concern `day-bit-other-than-zero-or-one`). A day of a period without a
  bit is available, as the schema says ("If bit is missing, assume
  available"), where the crate read it as not; a bit past the period's last
  day gives no day. Counted over the deliveries at hand, all 520 726
  periods with day bits state one bit for each day, none with white space
  or another character: the schema's reading changes none of their days.
- **model**: `Date` checks the length of its month: `2026-02-30` was
  2 March. A period bounded by such a date, an operating day and a day type
  assignment dated so give no day, a finding of a new concern
  (`Concern::DateOfNoDay`, `date-of-no-day`); none of the 10 600 576 dates
  of the deliveries at hand is one. `Duration` reads its days, then after
  `T` its hours, minutes and seconds, each at most once and in that order, a
  fraction on the seconds alone, white space around it set aside, without
  overflow, else none: `PT1.5H` was an hour, `PT1H1H` two,
  `P999999999999999D` overflowed. None of the 3 085 870 durations at hand is
  such a form.
- **model**: `MultilingualString::default_text`, the text where no language
  is asked for — its own, else the translation stating no language, else
  the first —, which the index and netex2gtfs read names by: a NeTEx 2.0
  name stated as `Text` elements alone was empty. A text beside the
  translations, which 2.0's mixed content allows, reads and writes
  (`xml::Cursor::next_child_with_text`, `xml::Emitter::text`), where the
  document was refused. `VehicleMode::Other`: the types of most elements of
  a mode list `other`, which read as `Unrecognised` (13 886 in Sweden's
  delivery).
- **crs**: the corrections of MGI and LV03 hold in the cells of a tenth of a
  degree holding a stop of their registers (`src/crs/areas.rs`, written by
  `tools/crs_areas.py`); beyond, the polynomial extrapolated — a position
  in Stuttgart in LV03 landed 302.6 m from itself in LV95 — and the Helmert
  transformation alone stands, within two metres. No delivery at hand
  writes either system.
- **merge**: a journey kept once points at the earlier delivery's journey
  under the identifier the merge writes it with, and the finding names it
  so: one alike with a journey the delivery before renamed pointed at the
  other journey of the old identifier. An object renamed takes along each
  identifier within it an earlier delivery holds — the passing times of a
  journey, the points of a pattern —, with its references, where it stood
  twice in the merge. From ten deliveries on the rank is written in as many
  digits as the last takes (`01-oebb/…`), so that the tenth no longer sorts
  before the second and answers ahead of it.
- **referrers**: `Referrers::journey_of` and `line_of`, the journey and line
  an object is, in constant time.
- **fuzz**: a dictionary of the forms read by their lexis
  (`fuzz/netex.dict`), which CI passes to every target.

### netex2gtfs 0.6.0

Built on netex-rs 0.12.0. Names of lines, levels, numbers for advertisement
and data sources are read by `MultilingualString::default_text`, so that a
NeTEx 2.0 name stated as translations alone is not empty.

### netex-cli 0.7.0

Built on netex-rs 0.12.0 and netex2gtfs 0.6.0. `netex validate` takes each
object's journey from `Referrers::journey_of`, where it searched every
definition for each journey: Austria's VOR in 7.9 s where it took 158 s (on
eight threads, before and after in turn), Switzerland in 81 s and Germany in
118 s, which did not finish within twenty minutes before. The report's schema lists the classes
`time-out-of-range`, `day-bit-other-than-zero-or-one` and `date-of-no-day`
(layout version 1, unchanged).

## 0.11.0 — unreleased

A delivery is read as its publisher's claim: no member is written outside a
merge or an extract, none ends the process by the size it claims, a period
costs the index the days it gives and no more, and what is read and written
is well-formed XML to the innermost attribute. Breaking, and so the next
minor version: the enums that grow match with a wildcard arm, and the
interfaces named below change.

- **dataset**: a member's name is listed as written, and its document is
  refused when loaded (`Kind::DocumentName`) where the name is no path of
  plain parts within the source — rooted, naming `..`, an empty part or `.`,
  holding a backslash or a control character, a file name not in UTF-8, or a
  name the archive gives two members. `merge::merge` and `netex extract`
  wrote such a member where its name led: `../../x.xml` two levels above the
  directory written to, an absolute name at that path. Of two members of one
  name the first was lost without a word; both are refused now.
- **dataset**: a document is read no further than its source's limit,
  `DEFAULT_MAX_DOCUMENT_BYTES` (4 GiB) unless set
  (`Source::with_max_document_bytes`), and refused as `Limit::Document`
  beyond: a member claiming more unread, one inflating to more when it does.
  The room for a document is reserved fallibly; before, a member claiming
  2^50 bytes ended the process (`memory allocation … failed`). The largest
  documents of the national deliveries read so far: Belgium's railway
  network 1 084 707 209 bytes, Switzerland's largest timetable 743 670 430,
  Sweden's shared data 711 094 297.
- **dataset**: a directory reached by several paths — a link into itself, a
  link `current` beside the directory it names — is read once, under the
  first path in the order of the names; before, a link to `.` listed every
  document 41 times and two such links failed the listing. A file is
  fingerprinted through a buffer rather than read whole. An entry loaded from
  another source than the one listing it is refused rather than read as that
  source's document at its position. Positions are read with `seek_read` on
  Windows, where the crate did not build (`std::os::unix`); CI checks the
  build for `x86_64-pc-windows-gnu`. `dataset::SharedFile`, public without a
  way to make one, is not (breaking).
- **dataset**: a manifest's text form writes a backslash, a tab, a line feed
  and a carriage return in a name as `\\`, `\t`, `\n`, `\r`, so that every
  name reads back as it was written; a backslash beginning no escape stands
  for itself, as in a manifest written before. A name holding a backslash
  and `t`, `n` or `r`, written by an earlier version, now reads as the
  character escaped.
- **calendar**: `period_days` takes the span of days asked for and gives the
  period's days within it (breaking); a period to 9999 nothing names cost
  23 MB and 2.9 million steps however few days the index held, one to the
  year 2147483647 ended the process. A calendar holds the years 1 to 9999
  (`FIRST_DAY`, `LAST_DAY`): an operating period, an availability condition,
  an operating day, a day type's validity or an assignment's date reaching
  beyond is read to the end of 9999 and is a finding, a new concern
  (`Concern::DaysOutsideCalendar`, `days-outside-calendar`).
- **index**: a calendar keeps the words of its own days alone, not a bit for
  every day from the delivery's first day to its last: a period of 1900 to
  9999 one day type is assigned gave every day type, period and condition a
  field of 2.96 million days; it costs that day type's calendar now. The
  journeys run on the same days. `IndexBuilder::merge_all` takes the day the
  builders setting one are built for (`at`) — it kept the largest builder's,
  set or not — and panics for builders built for different days.
- **xml**, refusing what was read before: the reader refuses what is not
  well-formed XML 1.0, in an element skipped or carried through untyped as in
  one it reads — an attribute written twice, unquoted or without white space
  before it, a `<` or a reference to an undefined entity in an attribute
  value, a character XML 1.0 forbids (below U+0020 but tab, line feed and
  carriage return; U+FFFE, U+FFFF) written or referenced anywhere, character
  data or an end tag before the root, an XML declaration other than at the
  start. Such a document was read, and its faults written back by `write`,
  `merge` and `extract` for other parsers to refuse; it is an error of the
  document now (`Kind::Xml`, `Kind::Entity`). README promised as much.
  Every national delivery at hand — 21, of ten countries — reads as
  before, to the same statistics. The checks cost the lean reader about 5 %
  on one thread (Austria's VOR, documents in memory); `netex stats` of
  Austria's VOR, Switzerland and Germany on sixteen threads took as long as
  before within the spread of a shared host.
- **xml**: the emitter writes a carriage return as `&#13;`, and in an
  attribute a tab and a line feed as `&#9;` and `&#10;`, so that a value set
  with them reads back as it was set rather than with spaces; a character
  XML 1.0 forbids, which no document can hold, as U+FFFD.
  `written::edit::attribute_value` writes values alike, and `>` as `&gt;`.
  `Emitter::end` without a `start` panics saying so, where it overflowed.
  `Emitter::declaring_netex`.
- **lib**: `write_element` declares the NeTEx namespace on an element that
  does not declare it itself — one taken from a delivery —, which
  `read_element` refused to read again; `Document::write` likewise.
  `Kind::NotNetex` names the element expected (`expected`, breaking).
- **version**: `rewrite` keeps a value written between single quotes between
  them — a value holding `"` made the document written for the other release
  not well-formed —; reads each element's namespace where its declarations
  state it, so that content of another namespace below the root is neither
  evidence of a release nor left out; and from where a document stops being
  well-formed copies it as it is and says so (`Report::malformed`,
  `Detection::malformed`, `Malformed`; breaking), where it stopped there and
  called the result lossless.
- **geometry**: a number that is no finite coordinate (`NaN`, `inf`) makes no
  position; `Geometry::positions` of fewer than two numbers to a position
  gives none, where it panicked.
- **The interface that grows** (breaking): `Release`, `index::StopKind`,
  `geometry::AxisOrder`, `version::Item`, `findings::Error`,
  `calendar::PeriodIssue`, `crs::Axes` and the entities of every container
  (`StopPlaceItem`, `JourneyPatternItem`, …) are `#[non_exhaustive]`;
  `xml::Child::start`, a quick-xml `BytesStart` read nowhere, is gone, so
  that a minor release of quick-xml is none of the crate's. The structs stay
  as 0.10.0 says. The methods taking an id say that an id of another index
  panics.

### netex2gtfs 0.5.0

Built on netex-rs 0.11.0. `Profile` and `StopIds` are `#[non_exhaustive]`
(breaking). The feeds are the same: Austria's VOR, Norway and Belgium's
railways convert to the same bytes as with 0.4.0.

### netex-cli 0.6.0

Built on netex-rs 0.11.0 and netex2gtfs 0.5.0. A report that cannot be
written — a full device — exits 2 rather than panicking with 101, and one
whose reader stops reading (`netex validate --strict … | head`) exits with
the verdict. `netex extract` and `netex merge` exit 2 for a delivery with a
member named outside it, writing nothing outside their directory. The
report's schema lists the class `days-outside-calendar` (layout version 1,
unchanged).

## 0.10.0 — 2026-11-04

Switzerland converts in 7.5 GiB on sixteen threads, not 16.9 GiB; a pattern
naming the link of other stop points is a finding; Bordeaux's courses are
its shapes; five managed objects type their codes.

- **index**: merging the builders of the threads takes over the strings a
  worker allocated rather than copying them — the copies were allocated anew
  while the worker's strings, freed, stayed with the allocator of the thread
  that had read them —, and a call's identifier is kept only where the
  vehicle passes through, the one finding naming it: the Swiss delivery's
  42 million call identifiers were nine in ten of its index's strings.
  Converting Switzerland peaks at 7.4 to 7.5 GiB on one to sixteen threads,
  before 12.4 GiB on one and 16.1 to 16.9 GiB on four to sixteen; Germany
  on eight threads at 11.6 GiB, before 12.7 GiB, as fast as before; the
  feeds are the same bytes (`docs/PERFORMANCE.md`). `Index::string_count` counts
  fewer strings where calls state identifiers.
- **index**: a service link a journey pattern names in sequence that leads
  from or to a stop point the pattern does not name is a finding about the
  pattern, naming the link (`Concern::LinkBetweenOtherPoints`, code
  `link-between-other-points`), where service links are read: the link the
  pattern's document defines, else the one of the document named first. 2 148
  patterns of Finland's delivery name such a link (`docs/INTERPRETATION.md`
  § 41). Every service link read keeps its ends for it.
- **model** (breaking: fields more on structs built by name):
  `ServiceJourneyPattern::links_in_sequence` (`LinksInSequence`,
  `ServiceLinkInJourneyPattern`), `ServiceJourney::link_sequence_projection_ref`,
  and `key_list` and `private_codes` on `DataSource`, `ResponsibilitySet`,
  `PassengerCapacity`, `AllowedLineDirection` and `HeadwayJourneyGroup`, all
  carried through untyped before.

### netex2gtfs 0.4.0

Built on netex-rs 0.10.0.

- **Shapes from a journey's own projection**: with shapes, a link sequence
  projection standing among a frame's members is read, and a trip's shape is
  the projection its journey names (`LinkSequenceProjectionRef`), where it
  states a geometry, else the service links it runs over as before. Bordeaux's
  delivery (TBM) states its courses so: all its 39 749 trips have a shape,
  within 0.007 m of Bordeaux's own GTFS (`docs/INTERPRETATION.md` § 42).
  The decision `projection-without-geometry` names a projection stating
  none; `shape-in-unknown-system` now speaks of links and projections.
  Read without shapes, the projections are among what the feed does not
  carry, as before.
- The links a pattern names are read from the typed model, the feeds the same.
- Merging what the threads gathered takes over their strings rather than
  copying them.

### netex-cli 0.5.0

Built on netex-rs 0.10.0 and netex2gtfs 0.4.0. `netex validate` reports the
class `link-between-other-points`, with the journeys running each pattern,
refusable as the others are; the report's schema lists it (layout version 1,
unchanged).

## 0.9.0 — 2026-11-02

What converting the national deliveries to GTFS showed, taken up: a time zone
from the data, shapes from the Nordic link projections, a feed the same bytes
on any number of threads, and the feed's archive deflated on every thread.

- **index**: the interchanges at a stop point are listed in the order of the
  names of the documents stating them, each document's in its order, whatever
  order the documents were read in — before, in the order the threads merged
  what they read, so that of an interchange two documents state otherwise the
  first listed depended on the threads. `Index::stop_version` and
  `Index::stop_point_version` tell the version a stop or stop point is stated
  in: two versions of one stop are two stops of the index, its identifier,
  version and name tell every one apart.
- **crs**: an `srsName` that is an EPSG code alone, `4326`, as Norway's and
  Sweden's link geometries write it, names its system
  (`ReferenceSystem::from_srs_name`).

### netex2gtfs 0.3.0

Built on netex-rs 0.9.0.

- **Time zone**: a delivery whose documents name none, given none, takes the
  zone of the country most of its journeys' identifiers show — `at:` now
  whatever profile its frames are typed as, so Austria's national deliveries,
  typed as the European profile's, take Europe/Vienna —, else of the country
  more than half of its stops with a position lie in, by Natural Earth's
  borders of the member states of the European Union and of EFTA, the United
  Kingdom and the small states among them (`src/countries.txt`, public
  domain; written by `tools/countries.py`): the Belgian operators' deliveries
  take Europe/Brussels, the French ones naming none Europe/Paris. The
  decision `time-zone-of-profile` is now `time-zone-of-identifiers`, beside
  the new `time-zone-of-stops`, each with how many journeys or stops show the
  country. A delivery whose stops lie in no country of the table still needs
  a zone given (`Error::NoTimeZone`).
- **Shapes from link projections**: a link's geometry is its `LineString`,
  else the one of its `LinkSequenceProjection`, where Norway's, Sweden's and
  Finland's deliveries state it, in the codespace's shared data. A trip runs
  over the links its pattern names in sequence (`linksInSequence`) — one to
  the next call each, or one for the whole way, as Finland's —, else over a
  link the delivery states between two of its stop points; each resolved as
  the trip's document sees it, then in the delivery. Before, links were found
  only by their two stop points and in the trip's own document. A link
  without a geometry is the decision `link-without-geometry`; a trip without
  a shape is `shape-incomplete` also where no link of the delivery has a
  geometry (before, nothing was said).
- **An empty web address is none**: `<Url/>` on an organisation is the
  decision `agency-without-url`, the publisher's web address given written
  instead; a data source with an empty one publishes no feed.
- **The same bytes on any number of threads**: the rows of `stops.txt` are
  written in the order of the stops' identifiers (stations first), those of
  `transfers.txt` in the order of the stops and stop points they start at —
  before, in the order the threads had numbered the stops —, and the first
  example of every decision follows.
- **The archive deflated on every thread**: netex2gtfs writes its zip archive
  itself (ZIP64 where a file or the archive exceeds 4 GiB), each file
  deflated in pieces of 1 MiB on the threads of the current rayon pool, each
  piece's compressor primed with the 32 KiB before it, the pieces one deflate
  stream; it no longer depends on `zip`. On 8 threads Germany converts in
  200 s instead of 364 s, Austria's nine deliveries in 39 s instead of 68 s,
  Norway in 16 s instead of 29 s; reading is as it was
  (`docs/PERFORMANCE.md`).

### netex-cli 0.4.0

Built on netex-rs 0.9.0 and netex2gtfs 0.3.0. `netex convert` needs no
`--time-zone` for the Austrian, Belgian and French deliveries any more; the
report says where the zone came from.

## 0.8.2 — 2026-11-02

Documentation and a benchmark; the code is that of 0.8.1.

- **README** written anew: what the crate and the `netex` command do, the
  five commands on the crate's own example, the profiles with their pages,
  what a GTFS feed written from NeTEx carries and what not, and what reading
  and converting whole national deliveries costs. The table of how each
  profile lays out its deliveries moved to `docs/PROFILES.md`.
- **docs/PERFORMANCE.md** begins with 34 national deliveries of twelve
  countries, each read and converted to GTFS on 8 and 16 threads, Germany on
  1, 4, 8 and 16: time, CPU time, peak memory and its course, the rows of
  every feed, shapes, and every feed held against the GTFS feed its publisher
  writes from the same data, with what the differences come from.
- **tools/bench/**, the benchmark behind those figures: it measures
  `netex stats` and `netex convert` on a list of deliveries under a lock
  against other measurements, compares the feeds with the publishers' and
  runs the GTFS validator of MobilityData on them. CI runs its tests.

What the measurements found, and this release leaves as it is: Austria's
national deliveries and the Belgian operators' name no time zone, so
`netex convert` needs `--time-zone`; a link geometry stated in the link's
projections, as the Nordic deliveries state it, gives no shape, and the
report does not say so; the order of the rows of `stops.txt` and
`transfers.txt`, and the first example of some decisions, depend on the
number of threads, the rows themselves do not; the feed's archive is
deflated on one thread, most of the time a large conversion takes.

### netex2gtfs 0.2.2

Built on netex-rs 0.8.2; its code is that of 0.2.1. Its README gives the
figures of whole national deliveries and says which link geometry gives a
shape.

### netex-cli 0.3.1

Built on netex-rs 0.8.2 and netex2gtfs 0.2.2; its code is that of 0.3.0. Its
README gives the figures of whole national deliveries and the time zones the
Austrian and Belgian deliveries need.

## 0.8.1 — 2026-11-02

Care after 0.8.0, compatible with it.

- **index**: a day type with a property of day naming no day — no days of the
  week, nothing more — is read as before, as every day of the week, and is a
  finding now, `Concern::PropertyOfDayWithoutDays`
  (`property-of-day-without-days`), made where the day type's properties are
  read: a delivery whose day types lost their days of the week runs every
  journey on every day of its periods, and only the finding shows it
  (`docs/INTERPRETATION.md` § 26).
- **CI**: the tests of the three packages run on what each package carries,
  the packages it depends on taken from their unpacked archives
  (`tools/package_tests.py`): a test reading what its package does not carry
  fails in CI, not first on the trial run before a release.

`privateCodes` on a passing time and on a stop point in a pattern: the schema
allows them none — both are versioned children, not managed objects — and the
documentation, which said otherwise, says so now (`docs/INTERPRETATION.md`
§ 40). Five managed objects whose keys the model does not type (`DataSource`,
`ResponsibilitySet`, `PassengerCapacity`, `AllowedLineDirection`,
`HeadwayJourneyGroup`) keep their `privateCodes` untyped, carried through as
written: a field more on a struct a caller may build field by field is not
compatible with 0.8, and is left for 0.9.0.

### netex2gtfs 0.2.1

Built on netex-rs 0.8.1; its code is that of 0.2.0. Its report counts the new
finding among the index's (`index_findings`).

### netex-cli 0.3.0

Built on netex-rs 0.8.1 and netex2gtfs 0.2.1.

`netex validate` reports the class `property-of-day-without-days`, with the
journeys running on the day type, and the check `stop-reference-to-nothing`:
of the references to nothing, those to a stop place, quay or stop point, each
with the element holding it and the journeys depending on it. A delivery may be
refused for them alone (`--refuse stop-reference-to-nothing`) and other
references to nothing tolerated. The JSON schema names both classes; the
report's layout is unchanged (`report_version` 1).

Not compatible with 0.2 as a library: `validate::Check` has a variant more. It
is `#[non_exhaustive]` now, so that a check added later breaks nothing. The
command line and the report are compatible.

## 0.8.0 — 2026-11-01

Four profiles more, each read in full on its publishers' deliveries, several
of them checked trip for trip against GTFS feeds published from the same data:
NeTEx France; the Nordic NeTEx Profile with the stop register it names; the
European Passenger Information Profile as Luxembourg, Denmark, Belgium and
Czechia publish it; and the Dutch profile with the national stop register (the
README's table of profiles, a page under `docs/` for each).

Not compatible with 0.7. Four of the model's exhaustive enumerations have a
variant more (`Frame`, `QuayItem`, `LineItem`, `PointInSequenceItem`); its
entities and frames have fields more (`private_codes` on most, every frame's
`content_validity_conditions`, the service and timetable frames'
`time_demand_types`, `Line::flexible_line_type`,
`ServiceJourney::dated_service_journey_refs`); `stream::Kind` places its new
kinds among the others, so its order changed. The index gives journeys of
earlier deliveries other days where it reads more of them: a day type's days of
the week within its validity, a period's bounds written as instants in UTC,
journeys dated by dated journeys.

Two readings are the crate's own choice where the standard leaves it open,
each with its reasons: when a day type's days of the week decide its days and
when its assignments do (`docs/INTERPRETATION.md` § 26); and that netex2gtfs
gives a French delivery naming no time zone none of its profile, France having
more than one — the caller names it (`docs/FR-PROFILE.md`).

netex2gtfs 0.2.0 and netex-cli 0.2.0 are published alongside, on netex-rs
0.8.0.

The French profile, NeTEx France (`docs/FR-PROFILE.md`): deliveries that write
everything in general frames, define quays beside their stop place, state
passing times without the point they are at and give day types their days by a
validity and days of the week are read in full. Five deliveries of the French
national access point were read before and after; the three whose producers
publish GTFS beside their NeTEx give the same trips, days and stop times
(`docs/INTERPRETATION.md` §§ 23–27). Not compatible with 0.7: `Frame` and
`QuayItem`, which are exhaustive, have a variant more.

- **model**: `GeneralFrame`, its `members` (`GeneralFrameMembers`,
  `GeneralFrameMember`) and `Frame::General`; `QuayItem::QuayRef`, a quay a stop
  place names by reference.
- **stream**: `FrameKind::General`; `Kind::Quay`, a quay standing on its own,
  in `Selection::routing`.
- **index**: a quay defined on its own belongs to the stop place naming it among
  its quays, else to the one its `SiteRef` names; a passing time naming no point
  is at the point of its position; an operating period without day bits gives the
  days of the week its day type names, and a day type with a validity none of
  whose assignments gives it a day of its own runs on its validity's days of the
  week, its assignments adding and taking away. New findings:
  `QuayOfTwoStopPlaces`, `PassingTimesOtherThanPoints`, `PropertiesOfDayUnread`,
  `WalkWithoutDuration` (a walk without a duration, before left out without a
  word). A stop place and its quay naming each other are no use of either in
  `Referrers::unreferenced`.
- **calendar**: `DaysOfWeek`, `Properties` and `properties`, the rule for every
  reader.
- Own example `tests/fixtures/own/fr/`, the fragment
  `quays-beside-their-stop-place.xml`.

The Nordic NeTEx Profile (`docs/NORDIC-PROFILE.md`): journeys dated by the
dated journeys naming them, flexible lines as lines, operating periods stated
by their operating days, and the stop register a delivery names read along
with it. Norway's national delivery was read before and after: 106 259
journeys dated only by dated journeys run on their days now, and its GTFS
conversion has, trip for trip, the days, stops and times of the GTFS feed Entur
publishes from it, but for differences each explained (`docs/INTERPRETATION.md`
§§ 28–32). A calendar no longer spans conditions no journey names: on
Norway's delivery `netex stats` took 356 s and 8.6 GiB, and takes 9 s and
1.8 GiB. Not compatible with
0.7 either: `LineItem` and `GeneralFrameMember` have a variant more, `Line`
and `ServiceJourney` a field more.

- **model**: `LineItem::FlexibleLine` and `GeneralFrameMember::FlexibleLine`,
  read into `Line`, which types `FlexibleLineType`;
  `ServiceJourney::dated_service_journey_refs`, the dated journeys a dated
  journey replaces as the Nordic profile names them (`DatedServiceJourneyRef`).
- **stream**: `Kind::FlexibleLine`, in `Selection::routing`.
- **dataset**: `Source::with`, several sources read as one — a delivery with
  the stop register it names, published apart —, the documents of each source
  after the first named under its file or directory name.
- **index**: a dated journey naming a journey and an operating day and stating
  no journey of its own gives the journey that day, unless it is cancelled or
  replaced; a flexible line is a line, and a journey whose line, route's line or
  pattern's line is flexible is left out (`Concern::FlexibleLineJourney`); an
  operating period stated by `FromOperatingDayRef`/`ToOperatingDayRef` takes
  their dates; the calendar spans only the days that can give a journey a day,
  and lists a calendar's days by its set bits.
- Own example `tests/fixtures/own/nordic/` with its stop register
  `tests/fixtures/own/nordic-stops/`.

The European Passenger Information Profile as four neighbours publish it
(`docs/EPIP-NEIGHBOURS.md`): Luxembourg's and Denmark's national deliveries,
the four Belgian operators' and the Czech national ones were read before and
after. Luxembourg and Denmark read as they did. The Belgian operators write
the bounds of their operating periods as instants in UTC, and every period
began a day early: STIB's and TEC's journeys now run, trip for trip, on the
days of the GTFS feeds the operators publish beside the NeTEx. The Czech
buses and city transport number their stop places from one in every
document: the bus delivery had 82 stops, every journey calling at another
document's; it has 98 129 now, each journey calling at its own document's
(`docs/INTERPRETATION.md` §§ 33–35).

- **calendar**: `bound_day`, the day a bound of an operating period names —
  from noon on, an instant with a zone names the next day.
- **model**: `DateTime::states_zone`.
- **index**: a stop place, quay or stop point is gathered by identifier,
  version and name; one another document names otherwise is a stop of its
  own, which the references of the documents defining it resolve to
  (`Concern::LocalIdentifier`; `Index::stop_by_id` names the stop of the
  document named first). A `StopPlaceRef` of a stop assignment naming a
  quay, or a `QuayRef` naming a stop place, is counted as resolved to
  another class.
- Own examples `tests/fixtures/own/lu/`, `dk/`, `be/` and `cz/`.

The Dutch profile, BISON's TMI9 NeTEx-NL (`docs/NL-BISON-PROFILE.md`), with
the national stop register published apart: six operators' deliveries were
read before and after. Before, every journey was left out — none lists its
passing times, and the calendar stood in a container the crate did not read;
now each takes its times from its departure and its time demand type, on the
days of its availability conditions. Converted to GTFS, RET's, GVB's and
Arriva's have every trip-day the national GTFS feed GTFS-NL has of them, with
its times but for differences each explained. Positions in RD New are
within 0.24 m of RDNAPTRANS2018 at every stop of the register. A stop's
global identifier is its register code where it has no `GlobalID`: the key
the German delivery names Dutch stops by (`docs/INTERPRETATION.md`
§§ 36–40). Not compatible with 0.7 either: `PointInSequenceItem` and
`GeneralFrameMember` have a variant more, and the entities a field more each.

- **model**: `PrivateCodes`, NeTEx 2.0's `privateCodes`, typed beside
  `keyList` (`private_codes`) but on a passing time and a stop point in a
  pattern, where it stays among the untyped children; every frame's `contentValidityConditions`
  (`content_validity_conditions`); `TimeDemandType` with `JourneyRunTime` and
  `JourneyWaitTime`, in service and timetable frames (`time_demand_types`) and
  among a general frame's members; `TimingPointInJourneyPattern` among a
  pattern's points, `TimingPointRef`; `Duration::milliseconds`.
- **stream**: `Kind::TimeDemandType`, in `Selection::routing`; the
  availability conditions of `contentValidityConditions`.
- **index**: a journey listing neither passing times nor calls takes its
  times from its departure and the time demand type it names — along the
  timing links its pattern names onward, through timing points that are no
  stop, waiting where the type or the pattern says; each pass of a link
  twice run its own run time, the waits of a point twice visited its last
  visits'; to the millisecond, each time rounded to the second. A link
  without a run time is a finding (`Concern::LinkWithoutRunTime`). A journey
  naming conditions none of which is available runs on no day. A stop's
  global identifier (`Stop::dhid`, `Index::stop_by_dhid`) is its `GlobalID`,
  else its `StopPlaceCode` or `QuayCode`.
- **merge**: stops pair by their register codes too.
- Own example `tests/fixtures/own/nl/` with its stop register
  `tests/fixtures/own/nl-register/`.

### netex2gtfs 0.2.0

Built on netex-rs 0.8.0, whose types its interface names.

A quay defined beside its stop place gives its platform code, level and
accessibility to its stop; train numbers are read wherever they stand, among a
general frame's members as well. A French delivery naming no time zone takes
none from its profile: France has more than one.

A delivery read with the stop register it names converts with its stops
(`Source::with`); journeys dated by dated journeys run on their days. Flexible
lines, typed now, are reported as not carried, as they were untyped.

### netex-cli 0.2.0

Built on netex-rs 0.8.0 and netex2gtfs 0.2.0.

`netex validate` reports the four new findings of the French profile
(`quay-of-two-stop-places`, `passing-times-other-than-points`,
`properties-of-day-unread`, `walk-without-duration`), the class
`local-identifier`, with the journeys calling at such stops, and the class
`link-without-run-time`; its JSON schema names all six.

`netex validate`, `stats`, `convert` and `extract` read documents along with
the source: `--with PATH`, a stop register published apart, as Norway's. With
global stop identifiers, `netex convert` writes a Dutch stop's register code.

## 0.7.2 — 2026-10-29

The README says what the crates are for: why official timetable data comes as
NeTEx, what makes a national delivery hard to read, what one builds with the
crate, and how to start with the library and with the `netex` command; it
carries the badges of crates.io and docs.rs. The code is that of 0.7.1.
netex2gtfs 0.1.3 and netex-cli 0.1.3 are published alongside, on netex-rs
0.7.2, with the same badges; netex-cli's README installs it from crates.io.

## 0.7.1 — 2026-10-29

The first published release. The code is that of 0.7.0; added are the
acknowledgements of the standard and its test data, a test that keeps the
repository's prose free of internal references, the packaging metadata for
crates.io and docs.rs, and the continuous integration of all three packages.
netex2gtfs 0.1.2 and netex-cli 0.1.2 are published alongside, on netex-rs 0.7.1.

## 0.7.0 — 2026-10-26

The merge of several deliveries is the library's: `netex_rs::merge`, which the
`netex merge` command of netex-cli 0.1.0 brought, so that whoever joins NeTEx
deliveries can call it without the command. Moved, not changed — the same rules,
findings and output; its tests moved with it (`tests/merge.rs`).

- **`merge`**: `merge(inputs, to)`, `Input`, `Report`, `DeliveryReport` — stop
  places paired by identifier, else by global identifier (`GlobalID`, `SLOID`),
  objects stated alike kept once, identifiers taken renamed into the delivery's
  codespace, journeys stated in two deliveries kept once (`docs/INTERPRETATION.md`
  § 16).
- **`written`**: a document as written — `scan` (where each element and the value
  of each attribute stand), `outline` (the objects of its frames, what they refer
  to and hold), `edit` (elements left out, values replaced, every other byte as it
  was) —, what the merge rewrites by and what a cut of a delivery needs.
- **`findings`**: the classes of findings with their counts and first examples, and
  the errors that stop a rewrite.
- **The feature `serde`** makes the reports serializable (`Report`, `Finding`); the
  crate needs no serde without it.

### netex-cli 0.1.2

Built on netex-rs 0.7.0: `netex merge` calls `netex_rs::merge`, `extract` writes by
`netex_rs::written` and reports by `netex_rs::findings`; the command, its arguments
and its reports are unchanged.

### netex2gtfs 0.1.2

Built on netex-rs 0.7.0; unchanged.

## Before 0.7.0

The versions before 0.7.0 were not published. In short, from the first tagged
version on:

- **0.2** (2026-10-04 to 10-11): NeTEx 1.x and 2.0 read and written into one
  typed model, everything the model does not name carried through as written;
  the releases told apart by the elements, attributes and enumeration tokens
  one declares and the other does not, and a document written for either with
  its losses reported; datasets read on every core; the index; the lean stream
  (`stream::read`, `stream::read_with`).
- **0.3** (2026-10-14): references resolved in their own document first; every
  decision of the index a finding, with a policy over their counts; the
  calendar rules (`calendar`); calls, availability conditions and template
  journeys (the Swiss form); passed points; the axis order of a link's
  geometry.
- **0.4** (2026-10-18): the package without the official schemas and examples,
  which stay test data of the repository; the licence files, own example
  deliveries under `tests/fixtures/own/`, the README for readers from outside
  and a CI workflow.
- **0.5** (2026-10-18): the repository a workspace with netex2gtfs 0.1.0 and
  netex-cli 0.1.0 beside the core; who refers to an object
  (`Index::referrers`); positions in national reference systems, the feature
  `crs`; own test fragments under `tests/fixtures/fragments/`.
- **0.6** (2026-10-20): the Italian profile of South Tyrol (IT-ITH1) read in
  full; a line named through a route; train numbers of a timetable frame and
  submodes beside a mode typed; netex2gtfs 0.1.1 and netex-cli 0.1.1 on it.
