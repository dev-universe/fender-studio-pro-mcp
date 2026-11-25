# Ampire host, stomp rack and tuner bypass

R353 used Studio Pro 8.1.1 build 113103 on macOS arm64. An exact Ampire
instance was the only insert on a muted empty stereo track. Main was
silent. The test checked the document, session, instance, transport,
tracks and mixer before changing any bypass.

| Control | Verified change and return |
| --- | --- |
| Host insert bypass | `false → true → false`. The sole slot and rack aggregate reported bypassed, and the editor's host activation light dimmed. The 90 non-meter rows retained values and became disabled. All 69 writable rows lost write access. The component fingerprint stayed the same while bypassed. Turning the insert back on returned the indicator and all 90 complete rows. |
| Ampire stomp bypass | `opt.stompbypass` changed `0 → 1 → 0`. Only the named typed row changed while the host insert stayed active. |
| Ampire tuner bypass | `opt.tunerbypass` changed `1 → 0 → 1`. Only the named typed row changed while the host insert stayed active. |

After each internal control returned, two independent reads and a fresh
component capture matched the original full non-meter rows and hash.
The four read-only meters were excluded from value equality and were
not written. Physical audio and tuner detection were outside the scope.

The first owner stopped after the verified host bypass write because
the test expected the single-insert rack aggregate to stay false. Two
read-only follow-up owners identified the parameter access
changes. A fresh owner then checked the observed bypassed state and
returned the exact insert once. The verified follow-up used 98
requests in 22.084 seconds. All four owners and the host exited normally.
The host ran 149.814 seconds. The scratch was not saved, and its original
bytes, 18 managed files and 10 touched app-profile files matched.
