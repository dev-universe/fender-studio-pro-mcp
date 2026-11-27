# Remove the empty third Splitter from a nested route

R424 ran on Studio Pro 8.1.1 Build 113103 for macOS arm64. The owned
scratch was a fresh copy of the R251 saved graph with eight nodes and
ten ordered successor links. Mixtool and Mixtool 2 began on separate
branches of Splitter 2. The first Mixtool fed Splitter 3, which led
from two empty lanes to its own merger.

The exact Splitter 3 node menu offered a single Remove item. It was
invoked once with transport stopped and Main muted at zero. The
Routing view showed the original two parallel Mixtools without a
third Splitter. The top Edit item recorded Undo Splitter Remove.
No Undo was invoked.

Two independent routing and ownership reads found six nodes and
seven ordered successor links. Splitter 3 and its paired merger
were the only removed nodes. The preceding Mixtool connected
directly to the original inner merger. Its successor was the only
changed retained edge. Node descriptions, retained graph and slot
identities, retained native child order, and both 72-row DSP
providers matched. Mixer state and the empty timeline were stable.

This evidence covers the native host menu with empty lanes. The
public `insert_splitter_remove` implementation has a separate
two-occupied-slot admission. Public command coverage and
persistence of this empty-lane shape remain open.

The owner used 86 requests in 78.977 seconds and released six
temporary state captures. It exited normally and the host closed
without saving after 126.741 seconds. The source and scratch
hashes matched. Eighteen installation files and nine application
profile files were verified after restoration.
