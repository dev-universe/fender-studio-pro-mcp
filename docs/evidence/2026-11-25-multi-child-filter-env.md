# Exact Multi Instrument child Filter Env recovery

R396 used Studio Pro 8.1.1 build 113103 on macOS arm64 and the candidate
source `fcf91fdc404cbebbf274ee899589645b62f544ad`. The native binary UUID
was `C29ACB79-7874-3440-92E9-6351F654E51E`. One owner opened the exact
two-Mai-Tai scratch with stopped transport and silent Main. The parent
and both live Devices were bound before any write.

The saved root Filter Env pilot started at 5449.993014335632. Each child
amount started at 5003.644824028015. A root round trip to 5000 and back
returned the root exactly and left both child amounts at the pilot's value.

The new `plugin_multi_child_filter_env_set` tool compared the exact
current Float64 bits and changed one resolved child per request.
The first child recovered its original bits while the parent and second
child remained unchanged. The second child then recovered its own bits.
Each call acknowledged its own change as `applied_unverified` and was
followed by two independent, complete reads. All 150 parent rows and
both 283-row child providers matched the baseline. Both slot and live
Device identities stayed the same.

A crosswired child identity, incorrect current bits, incorrect parent
identity, and wrong document were each refused before applying. Replaying
the first child's old bits was also refused. The fixture remained exact.
Offline tests exercise malformed or lost receipts after possible dispatch.
This run sent no deliberately lost request and saved no song.

The owner completed 144 requests in 11.934 seconds. The app ran for
53.025 seconds. Both exited normally. The trial song remained byte-identical,
and 18 managed files and nine profile files were restored.
