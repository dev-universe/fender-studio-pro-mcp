# Ampire linked microphone arithmetic

R425 traced the Ampire arm64 slice shipped with Studio Pro 8.1.1
Build 113103. A fresh extraction matched the slice from R352 byte
for byte. The arm64 UUID is `E83B8167-CF19-3B5B-A5C6-E7EBD743AB31`
and its SHA-256 is `a6f391f97b20f872ef2614eca7af8cd8d68c541bedc34ecba2fd57cbd0379027`.
REA/Ghidra 12.1.4 provided twelve selected procedures and the
Mach-O fixups identify the cabinet controller setter table slot.
Craft Anything 0.6.1 Deep independently decoded the setter,
configuration update, processor message sender and receiver.
Nineteen Craft arm64 instruction anchors match image bytes.

## Controller and processor paths

| What | Image offset | Observed behavior |
| --- | --- | --- |
| Declaration | `0x9e60` | Parameter IDs 3, 4, 5 and 6 register Link and A/B/C. The installed Ampire declarations expose the three gains as storable values and the enabled flags as read-only. |
| Parameter setter | `0xce7c` | Separate controller fields hold the retained basis and the current gain vector. For an active cabinet with three enabled microphones, a linked setter keeps the selected coefficient and applies the remaining share to each peer by rounded operations. |
| Link | `0xd084` | Turning Link off copies retained enabled microphone bases to the current vector. Turning it back on anchors A and recalculates B/C. |
| Different final C pivot | `0xcff0–0xd00c` | When C is enabled and the last edited pivot differs from C, copy the current A/B pair to the retained basis before setting C and distributing its remaining share. |
| DSP publication | `0xce1c`, `0xd0e94`, `0x14140` | The controller sends a grouped update of the current cabinet vector and related fields to the processor. The registered processor receives the matching update and sets its update flag. The three calculated gain fields in the message are current gains. |
| Cabinet registration | `0x1bcf8` | Registers impulse sources and cabinet classes. The separate impulse selection and completion paths at `0xd924` and `0xd754` resolve cabinet data and can recompute the controller vector. |

The grouped controller-to-processor message is a publication path.
A public exact cabinet batch load or controller rollback entry was
unobserved. The validated generic component capsule load changed A's
least significant normalized bit even when all the other observed
settings stayed exact. R422 established exact ordinary Save and
restart on the untouched factory vector.

## Independent numeric model

For active, positive normal coefficients with all three microphones
enabled, the assembly computes a Float32 complement of the selected
value and a Float32 sum of the two retained peer bases. For each peer
it multiplies the peer base by the complement and divides that rounded
product by the rounded sum. Both operations round separately.

The model reproduces all three observed bits after R362's linked B
write to raw 2500 and its subsequent exact-A correction. A B pivot
using the original basis yields the observed R416 Undo vector for all
three rows. This numerical match leaves the GUI Undo call chain
unverified. The capsule's one-row divergence differs from the latter
vector.

The target gain sum is 1.0000000521540642. At the exact factory A
pivot the target peer sum would exceed the exact binary32 complement
by about 2.553e-7 relative. With positive normal intermediate
results, the upper rounding bound from the separately rounded sum,
product and division is approximately 1.788e-7. A final A setter
or a relink anchored to the exact factory A therefore cannot produce
both original peer bits in this range. Different pivots give other
rounding intervals. A final C setter with a prepared A/B basis yields
the exact three observed gain bits in the offline model.

## One bounded real-host test

Use a new owned exact scratch with the original MCM 800 / 4x12 M65
factory state, stopped transport, and muted zero-level Main. Require
all three enabled flags and the original 90 stable non-meter rows.
Record the existing capsule fingerprint and release its token.
A different model, instance, baseline, or independent readback stops
the run. Each row below denotes one allowed write using the exact
DSP binding. No repeated dispatch is required.

| Write | Required subsequent A/B/C normalized Float32 bits | Host raw Float64 write bits |
| --- | --- | --- |
| Linked B at raw 2500 | `3F2A120B / 3E800000 / 3DAF6FA9` | `40A3880000000000` |
| Link off | `3F4BB3E9 / 3E800000 / 3DD2212A` | `0000000000000000` |
| Unlinked A one normalized ULP below factory | `3F4BB3E8 / 3E800000 / 3DD2212A` | `40BF152396800000` |
| Unlinked B three normalized ULP below factory | `3F4BB3E8 / 3DD03F92 / 3DD2212A` | `408FC6B337200000` |
| Link on, with B as the last gain edited | `3F4BB3E8 / 3DD03F94 / 3DD2212C` | `3FF0000000000000` |
| Linked C at the exact original C | `3F4BB3E9 / 3DD03F95 / 3DD2212A` | `40900817BC500000` |

Confirm every stage from a separate complete typed read. Confirm the
last vector twice and independently compare the component capsule,
all 90 rows, the mixer/timeline, and exact instance. The recorded
raw differences are fixed for this fixture. Use no Undo or Save
during this test.

The final C operation is predicted to retain an A/B basis one
normalized ULP below each original coefficient. An additional Link-off
observation after the primary witness, in the disposable scratch only,
would directly separate visible gain restoration from hidden basis
restoration. Historical R351 Link off exposed the exact original
basis on initial factory state. The linked 90-row and component
comparisons define visible and serialized recovery respectively.
The predicted basis distinguishes future unlink behavior even if the
capsule fingerprint matches.

The calculation ran in a Python binary32 model and in a separate
native arm64 C arithmetic check. Both reproduce the five selected
bit vectors. The C check runs authored arithmetic only. This R425
investigation performed no Studio Pro or plugin execution.
