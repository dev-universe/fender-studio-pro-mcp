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
- Verify UI outcomes in the actual app before changing settings or assuming a cache issue.
- Use the installed pstack workflow and Codex adapter. Existing evidence and authorization carry forward.

