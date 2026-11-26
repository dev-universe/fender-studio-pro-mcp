# Moving an effect across lanes before a nested Splitter

On macOS arm64, Studio Pro 8.1.1 build 113103, one public
`insert_slot_move` request moved the existing Mixtool 2 from the second
lane of Splitter 2 into the first lane, immediately after Mixtool and
before Splitter 3. The request supplied the live graph UUID, source
effect and node UUIDs, and the exact Mixtool → Splitter 3 edge. The
optional `branch` field was omitted.

The Routing editor initially showed the two Mixtools in separate lanes.
After the request it showed Mixtool → Mixtool 2 → Splitter 3 on the
first lane and a direct return on the second. The operation receipt was
`applied_unverified`. Independent native graph reads agreed with the
editor and showed exactly three ordered successor lists changing.

| Node | Before | After |
| --- | --- | --- |
| Splitter 2 | Mixtool, Mixtool 2 | Mixtool, Splitter 2 merger |
| Mixtool | Splitter 3 | Mixtool 2 |
| Mixtool 2 | Splitter 2 merger | Splitter 3 |

The graph retained eight nodes, ten links, both empty lanes of Splitter 3,
its existing folder, both devices and slot IDs, and all 144 typed DSP
rows. Mixer, sends and timeline readbacks agreed. The native child
enumeration order changed.

R497 issued 85 requests in 71.199 seconds and released six temporary
capture tokens. The app ran for 103.972 seconds. The scratch song was
closed without saving, both processes exited normally, and 18 managed
files and nine profile files were restored and verified.
