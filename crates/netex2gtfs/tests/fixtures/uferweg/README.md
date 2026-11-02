# Uferweg

A small delivery written for netex2gtfs's tests: two documents of one
delivery, each defining stop places of its own and naming the other's, so
that what the feed writes cannot depend on the order the documents are read
in or on how many threads read them. Places, lines, operators and times are
invented; licensed as the crate is (MIT OR Apache-2.0).

- `uferweg-nord.xml`: stop places Hafen and Mühle with a quay each, line N
  with one journey, a walk from Hafen to the other document's Steg, and a
  guaranteed interchange from its journey to the other document's.
- `uferweg-sued.xml`: stop places Steg and Anger with a quay each, line S
  with one journey, a walk both ways between Steg and Mühle, and the same
  interchange stated again, as staying seated.
