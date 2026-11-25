# Multi Instrument ordinary CC facade coverage

R395 used Studio Pro 8.1.1 build 113103 on macOS arm64. The isolated
two-Mai-Tai test song used one registered receive-only Core MIDI source,
the track's All Inputs assignment, and only that track's monitor. Main
remained zero and transport was stopped. The source emitted CC0–119
except CC1, CC2, CC11, and CC64, which were already tested with both
children. The isolated loopback matched all 232 message bytes as ordered
three-byte messages.

Each of the 116 tested CC numbers changed its independently bound
`controlN` facade on at least two active reads. Two separate neutral
reads returned to zero. The complete set of 150 parent DSP rows matched
after each case. The earlier four cases bring the observed ordinary CC
range to all 120 numbers. This census observes the parent facade. It
does not identify a child target for each CC.

The final child scan found only the two `masterGain.gain` rows changed
from 1990.535855293274 to zero. A separate owner rebound the same
document and Devices and used the original exact raw bits once on
each assigned Vol 1 and Vol 2 macro. Each command restored its own
child and preserved the other child and the parent. Two full reads
then matched all 150 parent and 566 child rows.

The census used 1411 requests in 185.817 seconds. The gain recovery
used 83 in 7.509 seconds. The app ran 288.927 seconds. No song was
saved. The scratch bytes, 18 managed files and ten profile files,
including the original MIDI configuration, matched after normal
app, owner and temporary-source closure.
