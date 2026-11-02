# The benchmark of whole deliveries

`netex_bench.py` measures what reading a NeTEx delivery and converting it to
GTFS costs with the `netex` command, and checks each feed it writes against
the GTFS feed the delivery's publisher writes from the same data
(`gtfs_days.py`). The figures in the README and in `docs/PERFORMANCE.md` are
its output over `deliveries.json`. Python 3.9 or later, standard library only,
Linux (`/proc`, `wait4`).

## Reproducing the figures

1. Build the command: `cargo build --release -p netex-cli`.
2. Fetch the deliveries `deliveries.json` names — each entry has the address
   it was fetched from (`url`), its licence and its stand — into one
   directory, each under the path the entry gives. Two need unpacking: the
   Norman aggregate is an archive of archives, one per network, unpacked into
   `fr/normandie/<network>/`; the Dutch documents are gzipped, unpacked into
   a directory per operator, with the stop register (`NeTEx_DOVA_epiap_….xml`)
   in `nl/reg/`. Sweden's delivery needs a key from Trafiklab.
3. Measure, two runs of each step:

   ```sh
   python3 tools/bench/netex_bench.py run tools/bench/deliveries.json \
       --netex target/release/netex --data DATA --out OUT \
       --window 2026-11-02 2026-11-08 --validator gtfs-validator-cli.jar
   ```

   `--only ID` measures one delivery (a later call adds to the same `OUT`);
   `--lock FILE` takes an exclusive lock on the file for every run, so that
   measurements of several people on one machine do not overlap. The GTFS
   validator of MobilityData (`--validator`, a release's `…-cli.jar`) needs
   Java 17; on a national feed give it memory, e.g.
   `JAVA_TOOL_OPTIONS=-Xmx40g`.
4. `OUT/results.md` holds the tables, `OUT/results.json` every run with its
   resident memory over time; `netex_bench.py table OUT --list
   tools/bench/deliveries.json` writes both again from what was measured.

The window is the week the comparison counts trips and trip days over; it
must lie within every delivery's calendar and its publisher's feed's.

## Tests

```sh
cargo build -p netex-cli
NETEX_BIN=target/debug/netex python3 -m unittest discover -s tools/bench -v
```

They count a feed written in the test, measure processes of known size,
take the lock, write the tables, and — with `NETEX_BIN` — measure, convert
and compare two of the repository's own small deliveries.
