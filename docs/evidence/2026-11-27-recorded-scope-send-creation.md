# Recorded Scope sidechain Send creation

Studio Pro 8.1.1 build 113103 on macOS arm64. The trials used one MCP owner,
one protected scratch, stopped transport, Main muted and no active inputs.
The stock Scope Sidechain header was enabled before creation.

The recorded creation uses the exact sender's empty Send rack and the
independently retained and uniquely owned Scope input. The stock descriptor
layout, input facade, rack factory and unity-level operation were checked
against the installed arm64 image. Craft Anything independently matched nine
selected byte ranges to that same image. The edit adds a new Send at 0 dB.
The surrounding device and source rack are retained and checked again.

R578 used the trial and rejected a wrong Scope instance, stale document and
duplicate add. The verified add created one Send to the exact Scope. A single
stock Undo restored the empty rack. The trial made 69 requests in 71.089
seconds and the host exited successfully after 134.211 seconds.

R579 used the public core `d93980afa77f` with native UUID
`4D6D445A-2D66-33BF-947F-57C384C4E0DC` and fresh channel names. It used the
public read, create and remove tools in one session. The wrong instance,
stale document and duplicate creation were refused. Independent reads
confirmed one 0 dB unmuted Send, the exact destination, the bound removal and
empty Sends on both channels. The stock Undo menu recorded the removal and
creation separately. Undoing the removal restored the same Send UUID, level,
mute and routing. Undoing the creation then restored the original empty rack.
All 31 Scope DSP rows returned to the baseline.

R579 completed 78 requests in 115.223 seconds and the host exited successfully
after 164.377 seconds. The scratch closed without saving and its digest
matched. All 18 managed files and 10 app-profile files matched after
restoration. This trial covers the authored assignment, exact slot edits and
Undo.

## Saved assignment after a cold reopen

R580 used the same public core and companion add-on with one new protected
scratch and one stock Save. It established one exact 0 dB Send and independently
observed its unmuted mixer assignment before and after saving. The saved song
archive differed from the source and validated intact.

Studio Pro then exited and a separate process reopened the saved bytes with
writes to the scratch blocked. A fresh MCP owner refused the old document
token. Two reads observed the same Scope instance UUID and Send UUID. The mixer
reported the same destination, level and mute state, and all 31 Scope DSP rows
matched. The cold file digest was identical to the saved digest.

The warm phase made 60 requests in 50.646 seconds with one Save. The cold
phase made 11 requests in 1.794 seconds with no Save. Both host exits were
successful. The source scratch, 18 managed files and 10 profile files were
reverified or restored.

## Signal through the authored Send

R581B used the shipping core at `7fc42b48e579` with the same native add-on
UUID and a fresh scratch. A stock Tone Generator supplied the source while
transport was stopped and Main was muted. The Scope trigger used trace A,
which read Side Left alongside trace C. Trace B read Main Right. An earlier
attempt left the trigger on a silent Main input and was inconclusive.

At 440 Hz before creation, all three traces were flat. The public create
tool then authored one 0 dB, unmuted Send. An independent assignment read
confirmed the exact Scope input. The Scope editor showed periodic traces on
Side Left and a flat Main Right. Changing the Tone Generator to 880 Hz
approximately doubled the number of periods on the same 800-sample display.
The Scope editor supplied the signal observation.

The trial disabled the Tone Generator, removed the exact Send and confirmed
both Send racks were empty. It then disabled the stock Sidechain header and
restored all 41 tracked Scope and Tone DSP rows. R581B completed 130 requests
in 59.331 seconds and the host exited successfully after 204.550 seconds.
The original scratch digest, 18 managed files and 10 app-profile files were
verified after closing without saving.

## Native port ownership of the saved assignment

R584 traced the stock Send slot through its owned processor to the
AudioOutput and its typed connection call. The Scope input callback takes
that output. Both endpoint classes have count and indexed accessors for
their own connection lists. The comparison used 20 Ghidra queries, nine
loader rebases and eleven Craft Anything byte ranges from the same arm64
host image.

R585B used a private read-only candidate and reopened a copy of the R580
archive. Two observations matched the exact Send output to its slot and
processor, and the Scope input to its effect. The source was kind 0 and
owner-local index 0. The destination was kind 1 and owner-local index 1.
The lock provider field was empty on both observed ports. This establishes
the endpoint layout for this saved assignment. Direct reciprocal list
observation still requires an established synchronization and lifetime
contract for the two lists.

The trial made eleven requests in 1.768 seconds. The two native IDs,
31 Scope DSP rows and saved digest matched. The host exited successfully
after 37.704 seconds, with the 18 managed files and app profile verified
after restoration.

## Immediate Undo with a second source

R589, R590 and R591 used the same native UUID as R579. All three opened
write-protected copies of the saved R580 assignment and added an empty,
silent second source. The original source kept its exact Send UUID, level,
mute and destination. All 31 Scope DSP rows matched through the observations.

R589 observed two correct Sends after creation. A wrong-source Send UUID was
refused. Removing the exact second Send and undoing the removal restored its
UUID. The next Undo changed only the preceding fader and left both Sends.
A fresh MCP owner independently read both Sends and the changed fader.

R590 observed the existing fader Undo between setting the fader and creating
the second Send. Studio Pro then displayed the Send Slot Addition entry.
One Undo removed only the new Send. The previous fader Undo remained in the
menu, the fader stayed at its set value, and the original Send was unchanged.
R590 made 60 requests in 42.956 seconds with a 100.052-second host session.

R591 observed the missing addition entry before any attempt to remove the
second Send. The creation receipt reported a recorded edit, and independent
reads confirmed the exact second Send. Reopening the Edit menu still showed
the fader as the latest Undo. The planned stock-removal comparison stopped
at that point. R591 made 44 requests in 36.994 seconds with a 57.587-second
host session.

The observed request intervals between the fader and the new Send were
0.987 seconds in R589, 12.887 seconds in R590 and 0.960 seconds in R591.
The R590 run also opened the Edit menu during that interval. These trials
establish an immediate history mismatch and leave its trigger open. All
three closed without saving, matched both scratch digests and restored the
18 managed files and changed app-profile files.

R592 added a four-second delay with no intervening UI action. The observed
request interval was 4.987 seconds. The next Undo was Send Slot Addition.
It removed only the second source's Send and preserved the first Send and
all Scope rows. R592 made 59 requests in 23.443 seconds, with a 49.609-second
host session.

R593 and R594 used private diagnostic builds of the same source. REA and
Ghidra traced the action group and the host's retained Undo list. Craft
Anything checked nine byte ranges against the same arm64 executable.
Immediately after the native group finished, both traces showed a distinct
Send Slot Addition at the head of the list. In R594, a separate native
parameter edit was appended 27 host ticks later. It moved the Send Slot
Addition one position below the head. The first stock Undo then changed
only the second source's fader. Independent reads still found both Sends,
and Send Slot Addition was now the first Undo entry. Applying that Undo
removed only the new Send. The original Send and all 31 DSP rows remained
exact. R594 made 74 requests in 40.408 seconds with a 74.691-second host
session.

R595 repeated the rapid sequence using the uninstrumented core
`e371da9ed45a` and the same native UUID as R579. The two Undo actions
again changed only the fader and then only the new Send. R595 made 74
requests in 33.452 seconds with a 53.627-second host session. R592 through
R595 closed without saving, matched both scratch digests and restored the
18 managed files and the nine changed app-profile files per run.
