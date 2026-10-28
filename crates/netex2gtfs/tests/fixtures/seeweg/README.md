# Seeweg

A small delivery written for netex2gtfs's tests, in the shape of a Swiss or
Austrian line document: every file of a GTFS feed has something to come from,
and every decision the conversion takes has a case. Places, lines, operators
and times are invented; licensed as the crate is (MIT OR Apache-2.0).

- `seeweg-line-7.xml`: an operator with its address, a data source; stop place
  Seehof with two quays (one on level 0 and accessible, one without a position,
  both carrying the same GlobalID),
  stop place Bergdorf without quays and not accessible, within the parent site
  Bergdorf Ort, which carries the GlobalID as the Hessian files write it, stop place Talwil
  carrying a Swiss SLOID with a quay carrying a stop place's GlobalID, as the Hessian files write them; a stop point placed by no assignment but with a position of its own, and
  one with neither; line 7 with its colours; destination displays changing on
  the way; service links with their geometry in LV95 (EPSG:2056); journeys on
  the weekdays of two weeks but one holiday, one arriving past midnight; a
  template journey every 20 minutes and one whose last run is no whole number
  of intervals after its first; a journey whose times go back; a guaranteed
  interchange, stated a second time; a walk between two quays, stated again one
  way with a longer duration; a notice on a point of a pattern, which GTFS
  cannot carry.
- `seeweg-other.xml`: what a feed does not carry — a fare frame, a flexible
  line with a journey.
