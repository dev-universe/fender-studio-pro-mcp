# Engineering rules

- Preserve user songs and settings. Run live checks in a named scratch session.
- Keep raw user songs, audio, app logs, local paths and proprietary app resources out of Git.
- Keep the MCP server native Rust. Embedded JavaScript runs inside the DAW.
- Preserve upstream MIT notices for copied bridge and EditTask assets.
- Report verified results separately from accepted requests and unknown outcomes.
- Match request, bridge session and active document before applying mutations.
- Serialize DAW access across processes. A process-local queue is insufficient.
- Do not expose arbitrary eval, shell execution, or unchecked script requests.
- Never use codex in branch names or pull request titles.
- Open a separate pull request as soon as an independently usable unit is verified. Merge the verified head into main as soon as its required checks pass. Close superseded drafts and retain branches still needed for research or handoff.
- Verify UI outcomes in the actual app before changing settings or assuming a cache issue.
- Use the installed pstack workflow and Codex adapter. Existing evidence and authorization carry forward.

## Resuming paused work

- For continuation of the built-in and FabFilter plugin program, including `진행 재개해`, follow `docs/RESUME.md` to the current handoff branch. Read that branch's `docs/RESUME_STATE.json` and its `resume_document` before starting work. Keep dated handoffs as evidence and reuse completed units.
- An explicit pause stays in effect until the user requests a resume. A resume instruction authorizes continuing the documented queue within the existing scope.
- Revalidate changeable state and create fresh runtime bindings. Reuse immutable completed evidence instead of restarting completed sweeps.
- Report a checklist after each completed live unit, including actual elapsed time, completed scope, remaining counts, a rough overall remaining time range and why the estimate changed. Distinguish hardware-dependent checks.

