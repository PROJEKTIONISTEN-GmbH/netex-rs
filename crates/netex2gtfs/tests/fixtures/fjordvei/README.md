# Fjordvei

A small delivery written for netex2gtfs's tests, in the shape of a Nordic
NeTEx Profile delivery: the stop register, the shared data of a codespace and
one line file. Places, lines, operators and times are invented; licensed as
the crate is (MIT OR Apache-2.0).

- `_stops.xml`: stop places Torget, Brygga and Fjord with a quay each.
- `_FV_shared_data.xml`: the operator, the stop points and their assignments,
  the weekday calendar of two weeks, and the service links: their geometry in
  a `LinkSequenceProjection`, in WGS 84 named `srsName="4326"` with the
  `gis` prefix — as Norway's and Sweden's deliveries write them —, two
  between Torget and Brygga (the direct way and a detour), one from Torget
  to Fjord for the whole way, one without a geometry.
- `FV_FV-Line-1_1_Torget-Fjord.xml`: line 1 with six journey patterns, each
  with a journey: four naming their service links in sequence — the direct
  way on to Fjord, the detour, the way back on the link without a geometry,
  one link for the whole way to Fjord, as Finland's deliveries name them —,
  one naming none, and one naming a link between other stop points.
