# Rive

A small delivery written for netex2gtfs's tests, in the shape of a line
document of the French profile as Bordeaux's network writes it: general frames
only, and the course of each journey not on service links but in a link
sequence projection standing alone among the frame's members, which the
journey names (`LinkSequenceProjectionRef`). The projection's line string is
written as positions (`gml:pos`), latitude first, under `srsName="WGS84"`.
Three journeys name a projection with a geometry — two the same one —, one a
projection stating none, one names none. Their passing times name no point of
the pattern, as the French deliveries write them: the document is valid for
NeTEx 1.x, the release the profile is written for, not for 2.0. Places, lines and times are
invented; licensed as the crate is (MIT OR Apache-2.0).
