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
