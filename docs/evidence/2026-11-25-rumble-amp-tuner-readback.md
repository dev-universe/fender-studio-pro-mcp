# Rumble Native amp position and tuner readback

Trial R387B used Studio Pro 8.1.1 build 113103 on macOS arm64, one owned
scratch song, and one MCP owner. The host catalog, mapped vendor binary,
native class, insert slot, document, session, and object identity were checked.
The empty stereo track had no input or events. Its channel and Main were at
zero and transport remained stopped.

| Exact DSP control | Native kind and tag | Raw round trip | Editor observation |
| --- | --- | --- | --- |
| `global.amplocation` | Integer, 114716 | 0 → 3 → 0 | The visible input-to-output chain had no rendered blocks and its layout stayed the same. |
| `global.ampBypass` | Toggle, 114717 | 0 → 1 → 0 | The same empty chain remained visible. |
| `global.tunerBypass` | Toggle, 114718 | 1 → 0 → 1 | A direct write to 0 opened the Tuner screen. Restoring 1 returned to the chain view. |

Every reported transition was observed in two independent reads. The other
DSP rows matched the baseline, and the independent processor projection
preserved all 26 slot model identifiers and all 22 numeric settings. The
earlier R387 stopped before these writes because the full capture hash changed
between baseline reads despite identical DSP and processor projections.
R387B compared the explicit projection fields. No full-capture byte stability
is asserted.

The editor retained an asterisk after the writes. The final two reads matched
all 39 DSP rows and the projected processor state, and the trial song was
closed without saving. The scratch file was byte-identical, the 18 managed
files and 10 profile files were restored, and both processes exited normally.
The 66 owner requests took 73.686 seconds, including editor observation.
The host ran for 116.960 seconds.

The two amp writes prove control readback for this fixture. A populated chain
is required to establish visual amp and stomp order, the enabled effect of
amp bypass, and the model selection domain. The tuner observation covers
screen activation and dismissal with an empty input.
