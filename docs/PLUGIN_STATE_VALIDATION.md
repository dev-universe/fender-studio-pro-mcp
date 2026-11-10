# Plugin state validation

This follow-up extends the [previous 60-class validation](VALIDATION.md). It is
still in progress. Current-provider enumeration and representative controls do
not establish complete model, preset, nested-graph or writable-parameter coverage.

| Area | Actual host evidence | Remaining scope |
|---|---|---|
| Enumeration | 8,668 current-provider records / 60 class fixtures | Conditional and unexposed state |
| Finite selectors | 23 selectors / 298 changed choice cases | Empty Mustang/Rumble model catalogs and combinations |
| Preset catalogs | 59 discovery/release/refusal lifecycles | Broader observable preset recalls |
| Surround Panner | All 47 current writable fields | Other output layouts and audible behavior |
| Note FX | 257 selected fields; all 248 Arpeggiator/Repeater writable fields | Chorder model learning, commands and event roles |
| Impact | 4,227 direct fields plus two current-focus bindings | 31 Focus and two MIDI-controller roles |
| Other instruments | Mojito 32, Mai Tai 275, Presence 219 and a four-field SampleOne pilot | Remaining SampleOne fields, MIDI roles and macros |
| Nested graph | 42 Splitter controls and a two-Mixtool whole-folder roundtrip | Generic topology editing and other graphs |

Each completed live unit is counted only after its final full-provider
restoration checkpoint. In-progress cases and offline source tests are excluded.

## Actual 8.1.1 observations

The tests use Fender Studio Pro 8.1.1 Build 113103 on macOS arm64, one stopped
synthetic scratch song, muted empty fixtures and a single MCP owner. Native calls
remain pinned to the tested executable identities and official CCL 5.1.2 ABI.

- The full typed audit read all 8,668 current-provider records across 60 class
  fixtures. Direct Device parameter access supplied complete access metadata,
  including devices whose indexed controller count was zero.
- Pro EQ state capture, scalar change and capsule restoration passed. Two
  repeated cycles also passed rejection of a token on another Device and
  rejection after token release. All 199 provider values and their schema were
  compared independently.
- Seven further cases passed two change/load/readback cycles each: Ampire's
  microphone mix and amplifier model, Fat Channel's compressor attack and model,
  Voice FX detune, and External Effect Mono/Stereo input gain. After each load,
  two complete reads matched every writable value exactly and restored the
  complete baseline schema. Read-only drift was recorded separately.
- Pro EQ exposed a 74-entry native preset catalog. Applying one issued preset
  changed 20 parameter values. Capsule restoration then matched all 199 values
  and their schema in two independent complete reads. Both tokens were released.
- The subsequent preset runner enumerated complete catalogs and tested two
  issued indices each for Pro EQ and Fat Channel. All four recalls changed
  exposed writable values and restored the complete baseline twice. State and
  catalog tokens were released, and each released catalog was refused. Ampire's
  first two issued presets produced no exposed parameter change, so those two
  cases remain unverified despite accepted recalls and matching restoration.
- Six scalar/model metadata probes returned native precision, state flags and
  original-parameter observations. The observed model facades had Float values
  and no `IListParameter`; precision is not presented as a choice count.
- Focused graph inspection observed retained graph, node, slot and Device
  relationships. The probed function factory exposed generic EditFunctions
  metadata. This does not establish graph construction, connection or deletion.
- A separate private synthetic fixture rendered two nested Splitters, with one
  Mixtool on each inner branch. The nested control verifier passed 42 cases
  covering both Splitters' output counts, modes, gains, mutes and split
  frequencies. Each changed state and restoration required two complete fresh
  graph-control reads, including the sibling Splitter. Original active values,
  counts and modes were restored. The actual routing display confirmed the
  original nested structure afterward. Hidden inactive values and generic
  topology construction, connection and deletion remain outside this result.
- The second inner Mixtool passed a separate native gain change. Two complete
  reads showed only its gain changed while its sibling stayed unchanged; two
  further complete reads matched both 72-record providers exactly after
  restoration. Wrong-instance and stale-document writes were refused, with
  both providers independently unchanged. Optional parameter-owner inspection
  also returned bounded read-only controller metadata on this actual Device.

The first full scalar batch confirmed 100 parameter roundtrips across Binaural
Pan (4), Dual Pan (8), Mixtool (71) and Channel Strip (17). Expander then exposed
a coupled-state failure: individual scalar restoration passed, but its low/high
filter values differed at the complete-provider comparison. The runner stopped,
and the unsaved synthetic session was discarded. The follow-up verifier must
restore a frozen whole-plugin capsule before advancing between parameters.
The capsule-only follow-up also stopped: loading the saved state after changing
lookahead produced an exact attack-value mismatch. No tolerance was accepted.
A subsequent explicit capsule-plus-scalar correction run tested all 13 eligible
Expander candidates. It independently restored the exact captured attack value
after each accepted capsule load, then compared the full provider twice. Twelve
scalar cases passed. The remaining `swapfreqs` control reset its own value and
therefore failed the scalar change criterion. A separate actual action test
confirmed that one activation exchanged the two filter values and that a second
activation restored the complete 14-record provider and schema twice. This
source-backed button is now classified as a dedicated action. No numeric
restoration tolerance or baseline rebasing was used.
These are partial results; no all-writable acceptance claim is made here.

The later direct-digest rerun of the original four plugins passed all 100
candidates again. Unlike the first grouped comparison, every restoration in
this rerun required two fresh complete native writable-projection matches
before advancing; independent complete public pages also bracketed each fixture.

A first digest-based Pro EQ sweep stopped on its first candidate, `bypass`.
Two changed-value reads passed, but an accepted state-stream load left that
host parameter changed. Full-page diagnosis confirmed that only `bypass`
differed. No candidate was counted as restored. A subsequent explicit repair
for the exact Pro EQ class, DSP source and binary `bypass` role passed the
changed-read/load/direct-correction workflow. Fresh native digests and two
additional complete pages confirmed the immutable baseline, including exact
typed latch metadata. The extension does not admit other Toggle controls.

These state-stream omissions motivate a separate direct scalar path. It
restores each captured raw value and compares the entire exposed provider
before advancing. Model and preset workflows still require their separately
proved state restoration. A successful state-stream load is not assumed to
restore every host-facing parameter.

The first direct per-candidate run passed all 64 eligible Pro EQ scalar
parameters. Every candidate required two stable changed-value reads, exact raw
restoration, independent restored-value readback and a complete 199-record
provider comparison before the next write. Read-only differences were recorded
separately. This result covers these current scalar controls, not every model,
preset, action or opaque state.

The next per-candidate run passed all 51 eligible scalar controls across
Expander, Gate, Limiter and Tricomp. It compared the complete provider after
each direct restoration. Three dedicated actions remained separate, so the
combined run correctly retained a partial overall status.

The registered `swap_filter_frequencies` tool then passed actual Expander,
Gate and Compressor cases. Each first pulse exchanged the two captured filter
values in two stable complete-provider observations. Only that proof permitted
one inverse pulse; two further complete reads confirmed the original exposed
projection. Native acknowledgments and audible or opaque effects remained
unverified. The new direct-digest path also passed two Pro EQ scalar probes.

External Effect Mono confirmed 12 candidates, including literal notes, before
stopping at `scopeMode`. Restoring that control alone left `latencyOffset`,
`latencyTotal`, `pingMode` and `showImage` changed. Native hashes agreed with the
diagnostic page, while both value hashes still differed from the immutable
baseline. No further write or automatic recovery was attempted, and the scratch
changes were discarded. The diagnostic ledger now distinguishes agreement
with its current page from agreement with the original baseline. That run did
not verify coordinated restoration of these controls.

A fresh, disconnected Mono instance isolated this behavior: Scope-on changed
the Scope and image controls; Scope-off selected Setup and changed the offset
and derived total latency. Static callback analysis confirmed that these view
controls update one another. One captured image selection followed by one
captured offset restoration returned the entire 24-record typed projection to
its original baseline twice. No write to derived total latency was used. This
is one bounded observed recovery, not verification of every starting mode,
physical ping behavior or hardware calibration.

The next direct-digest batch passed 59 scalar cases across RedlightDist,
Bitcrusher, Phaser, Flanger and Chorus. Each restoration was followed by two
complete native digest matches; independent complete public-page projection
checks bracketed each fixture. All 64 records were inventoried, including five
read-only records whose changing observations were reported separately.
Another batch passed all 131 eligible scalar controls across X-Trem,
Autofilter, Beat Delay and Groove Delay with the same per-candidate digest
checks. Its complete inventory contained 140 records, including nine read-only
observations.
Surround Delay, Analog Delay, Room Reverb, Mixverb, Open AIR and Studio Verb
then passed all 186 eligible scalar cases using the same two complete native
digest comparisons after every restoration. The inventory contained 376
records, including 190 read-only observations. These three batches add 376
verified scalar cases to the earlier 115 per-candidate Pro EQ/dynamics cases;
this is still partial coverage of the complete writable inventory.

A later Compressor run confirmed four scalar cases before `studiocomp.autogain`
left `studiocomp.gain` changed after direct restoration. The schema matched,
but the complete writable value projection did not. The runner stopped before
the next parameter and the unsaved scratch session was discarded. A subsequent
run passed the other 17 scalar controls by direct restoration. The automatic
gain case then passed capture/change/load and two complete native writable
projection matches. No captured-scalar correction was dispatched: the saved
state itself restored the manual gain. All 18 eligible Compressor scalar
controls are therefore covered through their respective restoration paths;
the separate minimum-history reset action remains unverified.

The following direct-digest sweep completed 226 further scalar cases:
Multiband Dynamics (65), De-Esser (7), Rotor (13), Fat Channel (45), Voice FX
(4), Vocoder (74) and Vocal Tune (18). Each confirmed case restored its exact
captured value and matched the complete writable projection twice before the
next parameter. The final Vocal Tune scale case stopped: restoring the Custom
selector alone left seven note toggles changed. The unsaved session was
discarded. A fresh explicit integral scale selection then passed two
change/capsule-load cycles, comparing the complete 26-record provider and every
writable value and rejecting the released token. The earlier fractional List
candidate was not accepted as an
exact requested scale choice.

The disconnected External mode verifier subsequently passed six transitions
each on Mono and Stereo. Setup, Scope and image selections required two fresh
changed observations and two exact complete 24-record baseline matches after
restoration. The restoration sequence also recovered the captured `pingAgain`
availability profile before restoring image and offset values. Its only
permitted temporary metadata change is that exact button's consistent all-on
or all-off availability group. Unrelated metadata drift still stops the run.
These 12 cases establish the tested disconnected transitions, not physical
routing, ping emission or hardware latency calibration.
The separate coordinated-control run then passed `latencyOffset`, `mathMode`
and `zoomOut` on both Mono and Stereo, six further cases. Each own value changed
in two complete fresh reads and returned to the exact original full 24-record
projection twice after finite captured-value restoration. These runs made no
derived-total, port, calibration or state-stream writes. Physical ping behavior
and audible effects remain outside these disconnected observations.

A subsequent ordinary-control run passed all 18 selected candidates across
Mono and Stereo: eight numeric controls and the literal notes field on each.
Every direct restoration required two fresh complete native writable-projection
matches, with complete public pages bracketing each fixture. The overall run
remains partial because coupled modes, dedicated roles and unsupported fields
are handled separately; it does not certify every writable External record.

The exact-role literal verifier then passed `deviceName` and `deviceNotes`
on both External variants, four cases with eight actual writes. A fixed Korean
suffix was read back twice, then the captured original text was sent once. Two
further full typed reads matched all 24 original records and their schema.
No mode, latency, port, calibration, unit-label or capsule write was made by
this workflow. These instance annotation checks do not establish global
settings safety or physical hardware behavior.

The stronger Surround Panner direct check stopped after five confirmed cases:
restoring `soloLfe` itself left `sendLfe` and two speaker-mute values changed.
Its unsaved changes were discarded. A fresh capability observation established
the exact alias's `IStorable` support. Capsule restoration then passed the Solo
case and a subsequent sweep of all 47 exposed writable panner parameters.
Each candidate required changed readback, capsule load and two fresh complete
native digest matches, with independent full public-page checkpoints. This
covers the current fixture's exposed values, including the advertised speaker
mute fields; it does not establish every physical output layout or audible
speaker behavior.

Further direct-digest checks passed 71 selected scalar cases: Scope (29),
Console Shaper (7), IR Maker (5), Level Meter (4), Phase Meter (1), Spectrum
Meter (8), Tone Generator (10), Tuner (4) and VU Meter (3). Each restoration
required two complete fresh native writable-projection matches and independent
public-page checkpoints. Two Scope trigger controls and two Level Meter reset
controls were explicitly withheld from these scalar cases.

The subsequent Note FX direct-digest sweeps passed 257 selected controls:
Input Filter (4), Chorder (5), Arpeggiator (107) and Repeater (141). The latter
two include their persistent GUI page selectors and every currently exposed
writable field on those two fixtures. Every candidate required exact captured
value restoration and two complete writable-projection matches before advancing.
Independent full public pages bracketed each fixture. The 256 keyboard note-event
carriers and 135 Chorder learning, audition and command controls remain separate
workflows; these results do not establish their event or chord-state behavior.

The finite-model verifier passed all current choices for Fat Channel's EQ and
compressor selectors and Voice FX's selector: three baseline choices were
observed, and nine other choices were changed and restored. Every choice reused
its immutable state capsule, required two exact model reads and two complete
typed restored projections including normalized values and display strings.
Released state tokens were independently refused. This does not enumerate
combinations of several selectors or prove audible/opaque model behavior.

Ampire's five nonbaseline amplifier choices also passed their changed/restored
projection comparisons. The enclosing case stopped at the final released-token
check because a read-only peak value's normalized numeric shadow disagreed with
its exact Float32 metadata. The raw metadata remained authoritative; the batch
was not labelled complete. A local reproduction traced the two-double-ULP
difference to JSON number parsing. No comparison tolerance was increased.

Both Rust JSON boundaries now use exact floating-point roundtrip parsing.
The rebuilt actual-host run completed all 18 Ampire/Pedalboard selector cases:
18 baseline choices were observed and 262 other choices were changed and
restored, including all amplifier/cabinet choices and all choices in each of
the eight pedal slots of both plugins. Final released-token checks also passed.
Mai Tai's character selector then passed its baseline observation and 12 other
choices. Together with Fat Channel/Voice FX, these runs cover 22 current
exposed selectors and 283 nonbaseline choice roundtrips. Their complete model
domains, simultaneous selector combinations and audible effects remain separate
claims; Mustang/Rumble's placeholder selectors are not included.
Vocal Tune's scale selector subsequently passed all 16 current choices: one
baseline observation and 15 changed/restored choices, including the final
released-token checks. This brings finite exposed selector coverage to 23
selectors and 298 nonbaseline choice roundtrips. The note-enable peers were
included in every complete typed restoration comparison.

The next build also exposed fresh native parameter digests. Independent Python
projection from complete public pages matched all three native hashes for
Pro EQ's 199 records and Impact's 4,262 records, with three fresh native scans
per provider. The digests cover exact typed extrema, access metadata, native
values, normalized representations and display text. They remain sequential
observations of exposed parameters; opaque plugin contents are not verified.

The ongoing Impact sweep has confirmed 4,227 distinct direct scalar roundtrips
against its complete 4,262-record provider. This count accepts only the last
complete public checkpoint from the interrupted initial run and disjoint later
cases whose fixture-end checkpoints passed. The current persistent-field partition
contains 3,714 names and is complete. All 512 ordinary pad fields and the bypass field also passed,
including both sample offsets, pan and level. The latest 385-field unit completed
in 13 minutes 50 seconds. Every case required two fresh native full
writable-projection matches before advancing and a final public-page checkpoint.
The separate current-focus cutoff and sample-start-offset results are outside
this direct-field count. Both observed pad0 bindings passed two complete changed
reads and two complete restored reads. Other pad selections, 31 remaining Focus
roles and two MIDI-controller roles remain unverified.

A separate finite instrument sweep has completed all 32 selected ordinary
Mojito fields, all 275 selected Mai Tai fields and all 219 selected Presence
fields. Each case changed
one field and restored its exact original carrier, with two fresh complete
native projection matches before advancing and complete public provider
checkpoints. The full providers contained 37, 283 and 241 records respectively.
Mai Tai's overall runner result remains partial because its separately held
character-model selector is outside these scalar cases; all selected ordinary
cases and final restoration checkpoints passed. MIDI inputs, the unresolved
Mai Tai edit-view toggle, Presence script macros, saved-state persistence and
audible propagation remain separate claims.

SampleOne's four-field pilot also passed its full 235-record restoration checks.
A subsequent 124-field unit stopped after 122 individual restorations when the
app crashed during named readback following a gate-step edit. Its final provider
checkpoint did not complete, so none of that unfinished unit is counted. The
last setter had returned verified, followed by a successful fixture-context
inspection; the next read lost the session. The crash cause is unproved. No
automatic restore or setter retry followed. Reopening the saved scratch and two
complete 235-record observations recovered the exact original values and schema;
the saved file was unchanged. The failed unit remains separate evidence.
A later bounded read of the same name at its default value returned normally,
with two complete original-provider matches afterward and no parameter setter.
Static binary and exact SDK/compiler evidence identifies a routine in the crash
stack as global scripting garbage collection. This identifies a stack phase,
not the corrupting code or a verified crash fix.

The opt-in Music fixture context route also passed actual read-only preflight,
four mutation pilots and a subsequent 124-case Impact batch. Each check brackets
native inspection with fresh document, stopped transport, complete empty-track,
exclusive-selection and mute/arm/monitor observations. The independent write
guard additionally observed the scratch Main fader at zero before each setter.
This is a sequence of fresh observations, not an atomic snapshot or continuous
output guarantee. The legacy separate-read guards remain the default.

## Exact values and settlement

Native Float reads include `native_value.data.bits_hex`. An explicit
`float_bits` write preserves that finite f64 representation across JSON. The
audit found 38 JSON numeric representations differing by one ULP from native
bits; the verifier checks this measured bound and uses native bits for identity
and restoration. An affected Expander value passed an actual exact-bit roundtrip.

Ampire also demonstrated a different behavior: an immediate setter value settled
to another raw Float representation while retaining the exact same native
normalized value. Acceptance requires two fresh reads with stable exact raw bits
and normalized value, matching metadata and an actual change from the baseline.
Only Float values can use this normalized proof. Restored values still require
exact baseline identity; this is not a numeric restoration tolerance.

By default, the state and scalar verifiers compare every readable native value.
The explicit `--read-only-observations` option records read-only differences as
`observed_not_restored` only when complete native access metadata on both sides
proves `readOnly=true` and `writable=false`. It preserves the complete schema and
every writable-value comparison. Read-only output meters can change while
transport is stopped; those observations are not counted as restored state.

## State and preset contracts

`plugin_state` captures an independently copied in-memory `IStorable` stream.
It returns an opaque service/document/Device-bound token, byte count and digest.
It never imports or exports state bytes. Native load acceptance remains
`applied_unverified`; independent reads establish restoration. Opaque embedded
content and audible behavior are not certified by parameter comparisons.

`plugin_presets` lists a bounded issued catalog and applies only an issued index
on the bound target. The initial call uses a fixed empty URL prototype created
inside the bridge; callers cannot supply paths, URLs or state blobs. Catalog
origin is not inferred from read-only storage. Apply acknowledgment alone is not
an acceptance pass. The expanded native recall route covers the observed
AudioInsertSlot, AudioSynthSlot and MusicFXSlot families. In the actual host,
Pro EQ (74 catalog entries), Mai Tai (241) and Arpeggiator (10) each passed two
issued recalls with changed writable values, complete-provider restoration,
token release and released-catalog refusal. The latter two establish instrument
and Note FX acceptance on these fixtures. Other family members remain separate
acceptance cases.
The fourth observed slot family, AudioFXSlot, then passed catalog-only checks
for Surround Panner (zero current entries) and Console Shaper (13). Discovery,
release and released-token list refusal each required two complete typed
provider matches, six observations per fixture. Console Shaper also passed two
issued preset recalls with changed writable values and full restoration. No
panner preset recall is inferred from an empty current catalog.
The catalog-only verifier subsequently passed 59 class fixtures: 44 ordinary
audio effects, the panner and Mix FX fixtures, all seven instruments, all four
Note FX, and both External Effect variants. All three catalog lifecycle stages
required two complete typed provider matches each, including after the expected
released-token refusal. Empty current catalogs are explicit observations, not
preset-recall acceptance. The Splitter graph fixture has no ordinary Device
in insert slot zero and is excluded from this catalog result.
Mustang Native subsequently exposed a 227-entry issued preset catalog. Two
recalls changed its exposed output-level value and restored its complete
writable projection. Current category/effect model counts remained zero after
recall and restoration; two further complete projections matched the original
schema and writable values. This does not establish internal Fender model
selection or audible/opaque preset behavior.

Native17 list pages then observed the exact current preset display on Pro EQ,
Mai Tai, Arpeggiator and Console Shaper: all eight pages returned an observed
display, covering the four slot families. The existing catalog lifecycle checks
again preserved each full typed provider at all three boundaries. No dirty
status, issued-index identity, successful recall or host-label restoration is
inferred from the display.
A fixed read-only Inserts observation found the actual AudioInsertFolder
exposed `IPresetCollector` and `IPresetMediator`, with no `IStorable` interface.
Its object UID was present and class UID absent. The bounded auxiliary report
was partial; it did not establish whole topology, active-document ownership or
successful collection save/load. Two fresh Pro EQ native projections matched
its original full schema and writable values after the metadata observations.

The next native build observed the Inserts parent chain through its audio
channel, channel groups and AudioMixer to SongEnvironment. The six-node report
completed within its fixed eight-ancestor bound. No ancestor matched the active
document model's canonical identity. Two fresh complete Pro EQ projections again
matched its 199-record schema and every writable value. This establishes the
read-only parent route.

A subsequent native build resolved the fixed active-document controller and its
Environment child. In the actual host, that retained Environment's canonical
identity matched the Inserts ancestor chain. The original document-model
comparison remained false. Pro EQ's complete schema and all writable values
again matched twice after this separate read-only witness.

Native20 then passed the fixed Inserts preset preflight in the actual host.
The exact Device/slot/folder/channel attachments and current document Environment
witness were observed together with the actual target handler. Its flags were
3, and its retained metadata identified FX Chains, Fender and FXChain. The
folder object UID was present and its optional class UID absent. Pro EQ's 199
records again matched the original schema and all writable values twice.
This preflight performed no whole-folder file capture or restore; those calls
and independent graph restoration remain separate acceptance gates.

Native21 subsequently captured and reopened an actual host-authored 1,604-byte
single-Pro-EQ chain file. Capture preserved the original Device, full 199-record
schema, all 64 writable values and observed preset display. A same-state load
replaced the Device and matched the original schema and all writable values
twice, but its observed display changed from `default*` to `default`. The first
strict display-preservation gate therefore failed and its scratch was discarded.
The archive contains the part title `default`; it does not establish preserved
host dirty/UI state. The actual native reader observes the display string only.

A fresh, separately scoped run preserved that failed observation and permitted
only this fixture's declared display delta. One highcut value changed in two
complete 199-record reads; every other writable value remained unchanged.
Restoring the same original chain token once then matched the original schema
and all 64 writable values in two reads under a freshly resolved Device. Both
loads replaced the Device. Folder binding, slot order and exposed bypass
inventory remained unchanged, and the token was released. Read-only values were
observed separately. This proves this simple fixture's exposed-state roundtrip;
nondefault bypass storage, initial display/dirty/UI preservation, opaque
contents and nested topology remain separate claims.
A deliberate released-token request then returned native `not_applied` but an
outer MCP `outcome_unknown`, exposing an error-envelope mapping gap. Two more
complete reads preserved the original schema and all writable values. No further mutation followed the unknown outer result. A one-line bridge
normalization then passed 15 focused and 211 full JavaScript tests, independent
wire/Runtime checks, and a fresh actual-host regression. In that run the
released-token request returned outer `not_applied` with `native_rejected`; two
complete reads preserved the 199-record schema and all 64 writable values. No
load was accepted or repeated by this regression.

A separately declared nested-fixture run then captured and reopened a
2,859-byte two-part host-authored chain. One same-state load and one later load
of that same original token each replaced both Devices. Fresh positive
Graph-node-to-Slot-to-Device paths bound the two logical Mixtools after each
known accepted load; all six observed Device UIDs were distinct.

The run changed one inner Mixtool gain and then one inner Splitter gain. Two
complete reads proved only the intended child value changed, with its sibling
unchanged; a later pair also proved only the intended Splitter value changed.
The original chain restored both 72-record schemas, all 142 writable values and
all 14 originally active Splitter fields twice. The token was released and two
more full reads matched. The separately observed read-only values also showed
no changes. Initial and final child displays were both `default`, under the
predeclared exposed-state policy.

The root observed the upper and lower Routing views before capture and after
each load. Both nested branches, their merge, the outer bypass branch and final
merge/output remained visible in the same arrangement. Native graph reports
remain incomplete ownership reports and do not enumerate audio edges. Reads
and Main-zero checks were sequential; the capture's last Main observation to
receipt interval was about 31.4 seconds, without an enforced 30-second dispatch
bound. This does not prove atomic output blocking, physical silence, hidden
inactive Splitter values, opaque contents or generic node/connection editing.
Independent offline review rebuilt all 40 full-provider observations across ten
stages and confirmed the scoped lifecycle and intended deltas.

A new, globally empty one-track Chorder fixture exposed a separate persistence
failure. Its factory state had one explicit chord and 24 Auto Fill displays.
A bounded six-setter, three-Play/Stop preparation disabled Auto Fill, cleared the
model through the Mode-gated Clear All action, reset that command field and
returned Mode/selector to zero. Two complete 908-record reads proved the final
difference was exactly Auto Fill plus the original 25 nonzero assigned statuses.
This was a live model-status result, not a state-capsule restoration claim.

After Save and a full app restart, two reads instead matched the original
factory state. The saved Chorder preset member was byte-identical to its initial
member; only transport and Song metadata changed. Its preset dirty field was
already set in both archives, so a guessed dirty-flag setter is not established
as the fix. The failed reconstruction remains recorded separately.

A normal GUI Auto Fill edit produced the same 25-field live delta as the native
Auto Fill edit. Its saved preset member contained Auto Fill zero. A subsequent
full restart preserved all 908 typed values and schema in two reads. Public CCL
GUI controls bracket edits with beginEdit/endEdit; the current native numeric
path uses setValue(update=true) alone. This source difference and GUI contrast
justify a bounded edit-lifecycle pilot, but do not yet prove the missing calls
are the cause or that a native fix works. The fixed learning diagnostic itself
has passed 26 focused tests and the 559-test full Python suite; actual execution
remains pending on a suitable prepared fixture.

A subsequent native user-edit pilot tested that lifecycle hypothesis on the same
saved GUI Auto Fill-zero fixture. Exactly one Auto Fill-one edit dispatched
beginEdit/setValue(update=true)/endEdit and returned a checked live match in
83 ms. Two full 908-record observations matched the expected changed state,
and two further observations still matched it after Save. However, the saved
581-byte Chorder member remained exactly the original Auto Fill-zero member.
A full app restart and fresh instance resolution then matched all original
908 typed values and schema twice. The second planned setter was skipped.
The native phase/readback path works on this target, but bracketing this direct
receiver does not resolve its persistence failure. Its observed declared
controller is DeviceParamFolder and its original chain is self-referential;
these facts do not identify the GUI/model receiver or prove audible propagation.

A separate one-setter normalized-value contrast also returned a verified live
Auto Fill-one value in 88 ms. Two full 908-record reads matched before and after
Save, but the saved 581-byte preset member again remained byte-identical to the
original GUI-zero member. After a full app restart, two fresh full reads matched
the original values and schema. The second setter was skipped. Raw, normalized
and bracketed direct edits therefore share this observed persistence failure;
the cause remains unknown.

Read-only native metadata now identifies the Auto Fill parameter as DeviceParam.
The direct Device, parent slot and Presets child expose IController; the alias
does not expose IController or IControllerProvider. Two complete original
908-record observations followed each metadata check. These observations do not
establish that the GUI uses the same parameter or declared observer.

The finite opt-in implementation passed 90 parameter-library unit tests, four
documentation tests, 203 add-on tests and 76 Rust-server tests. The wire passed
220 JavaScript tests and an independent 266-case contract check. Six independent
native failure discriminators passed after callback/ownership corrections.
These code checks remain separate from the failed live persistence gate.

Chorder's learn-input selector also returned all 129 explicit input labels in
five read-only formatter calls. Two complete native projections matched all 908
original typed records and schema afterward. This establishes the formatter
route, without claiming that the selector-to-core-pitch binding or learning
model restoration has been verified.
Two subsequent learning pilots, including one with the normal editor visibly
open, stopped after enabling Learn mode: the fresh selector stayed at the valid
formatted sentinel row zero. The verifier had incorrectly required automatic
selection of a non-sentinel row. Neither pilot dispatched a learning-bit setter
or state load; their scratch changes were discarded. A minimal verifier fix
accepts the formatter-bound sentinel and retains the explicit key selection,
model-status and full restoration requirements. Its fresh actual-host pilot
applied Mode1, selected the explicit lowest input key, and set its fixed learning
bit. Two full reads observed the bit change while the independent explicit-chord
model-status field stayed zero. The model-binding gate failed; no state load or
follow-on cleanup was dispatched, and scratch changes were discarded. This
establishes the three value-write routes, not editing or restoring the chord
model. A processing/lifecycle explanation remains unproved.

A separate Impact focus-alias pilot observed stable, equal Float32 normalized
values but different exact Float64 raw projections for a generic test value.
Only the focus cutoff and its selected pad peer changed; both complete original
projections matched after the one original-alias restore. The original strict
alias equality gate therefore failed. A scoped verifier correction chooses an
exactly representable binary fraction for this cutoff profile and preserves
that equality gate. The fresh actual-host pilot passed: exactly this alias and
pad0 cutoff changed in two complete 4,262-record observations, their exact raw
carriers and bits matched, and one original-alias restore was followed by two
exact original full projections. This covers one current-focus binding and
current-value roundtrip. The other 32 aliases and other pad selections remain
separate cases.

`plugin_parameter_details` reports bounded original-chain facts without exporting
native pointers or invoking methods chosen by a caller. Node inspection is also
read-only and reports traversal limits. Neither API grants topology mutation.

A later exact-anchor EditTask probe added a fixed read-only observation of its
actual `context.editor`. Before and after opening and visibly focusing the
normal Routing editor, the bound Device, document and session were unchanged,
and both editor observations still reported `TrackListView`. The function
observation remained `EditFunctions`. This establishes the new metadata route,
but the unchanged task editor makes this focus experiment inconclusive about
a graph-specific context; it grants no topology mutation authority.

The subsequent `plugin_parameter_format` build successfully formatted explicit
inputs for Ampire amplifier/cabinet, Fat Channel compressor/EQ and Voice FX
models, without using a setter. Five queries returned 36 labels. Independent
complete-provider reads matched all writable values and schema before and after;
Ampire's read-only output meters drifted. This establishes those explicit input
representations, not inferred cardinality for every List facade.

Optional owner observations on Mustang Native, Rumble Native, Ampire and Pro EQ
returned non-Node `DeviceParamFolder` controllers. Opening the exact Mustang and
Rumble editors left each complete 39-record typed projection unchanged, with
the mutable effect-model placeholders still unpopulated. The underlying aliases
did report the declared `IControllerProvider` interface. A guarded getter observed
each alias's `RemoteRootComponent`, including `workspaceroot`, `global`,
`effect1` through `effect25`, `amp` and `tuner` child identifiers. Two fresh
complete 39-record projections were unchanged after these read-only calls.
The bounded type inventory was incomplete and established no model catalog,
occupied-slot count or selection API.
The fixed `plugInUnknown` virtual child then exposed each actual workspace,
including its `signalchain` and `presets` components. Both complete 39-record
projections again remained unchanged in two fresh observations. Depth limits
still prevented a complete inner inventory at this checkpoint.
The subsequent depth-five inventory observed `ToneEffectSelectorComponent`
and `ToneCabinetSelectorComponent` under each real `signalchain`, plus the
workspace preset component's four children. The full 39-record projections
remained unchanged twice. These metadata reads establish the fixed selector
route; model catalog contents and selection effects are separate checks.
The new read-only `plugin_models` operation then resolved both fixed model
objects successfully in the actual host. Their current category and effect
counts were zero, including Mustang after opening its real Add picker, which
was also visually empty. Both complete 39-record projections remained unchanged
twice after cold calls and after opening/cancelling that picker. Nonempty title
enumeration and model selection remain unverified; an empty current model is
not evidence that all available Fender models have been enumerated.
An unpopulated scalar facade is not classified as an empty rack.

## Reproduction and evidence

The runners include `scripts/verify_all_parameters.py`,
`scripts/verify_all_parameter_states.py`, `scripts/verify_plugin_states.py`,
`scripts/verify_plugin_presets.py`, `scripts/verify_plugin_actions.py` and
`scripts/verify_nested_graph.py`.
Finite model ranges use `scripts/verify_plugin_models.py`; disconnected External
mode transitions use `scripts/verify_external_modes.py`.
`scripts/verify_impact_focus.py` checks one fixed cutoff-alias binding.
`scripts/verify_chorder_learning.py` defaults to read-only selector preflight;
its explicit candidate mode tests one model binding before allowing the original
capsule load. Both dedicated mutation pilots remain pending actual-host
acceptance; their offline checks do not establish all-alias or all-learning
coverage.
Use their `--help` output for finite fixture,
scope and mode arguments. Start the MCP owner before launching the DAW, open the
named disposable scratch song, and retain the ignored append-only ledgers and
immutable baselines. Unknown dispatch, changed context or unconfirmed restoration
stops further mutations. A request acknowledgment never substitutes for a changed
value and independently confirmed restoration.

The tested Unit 4 server SHA256 is
`dc3a8a9b63f82d82542158150b90661f3748ac2fdfdc14ef01202357bb94a77e`;
the native add-on SHA256 is
`5535ab5178407e86c407b6e37ed854f0cdb081d6976f8108d58969b292314e4f`.
These hashes identify the tested artifacts, not every later source edit.
Raw receipts, tokens, values, machine paths, songs and vendor resources stay
outside Git. Source review, mocks and actual-host observations remain separate.
