# Restored Inserts chain after Save and reopen

R438B used Studio Pro 8.1.1 Build 113103 for macOS arm64, the exact clean R421 candidate `92145d5fe4ee546044a0c936224aaee31cc94266`, and an independent copy of the R155 test song. The initial R438 run stopped before a removal, restore, or Save because this older song opened at unity gain on Main. It exited after seven read-side requests and preserved both file hashes. The fresh R438B run verified stopped transport, muted Main, set its gain to zero, and independently checked the mixer before proceeding.

| Phase | Evidence |
| --- | --- |
| Capture and remove | One retained capture recorded the original six nodes and seven ordered links. One `insert_splitter_remove` request produced four nodes and four links with both Mixtools in series. Independent readback preserved the original slot and device bindings and all 144 typed DSP rows in the reduced graph. |
| Restore | One `insert_chain_state` restore with the retained token was accepted. Fresh reads and the Routing editor recovered the parallel six-node, seven-link shape and all 144 typed DSP rows. Both original slot and device bindings were replaced. |
| Save | After releasing the original token and all independent captures, the operator saved the scratch once. Independent parsing matched the restored graph and both newly bound slot IDs. |
| Fresh process | Fresh document, folder, and graph bindings were issued. The restored graph, node, slot and device IDs and all 144 typed rows matched. One restore request carrying the original document and token was refused before dispatch. Independent reads before and after matched, and the Routing editor showed the same parallel rack. |

The two owners handled 123 requests in 61.641 seconds and 62 requests in 29.284 seconds, and released nine and four temporary capture tokens. The hosts ran for 101.792 and 74.028 seconds and exited normally. The operator declined the reopen session’s Save prompt. The protected source and once-saved file hashes, 18 managed runtime files, and nine profile files were verified at closure.
