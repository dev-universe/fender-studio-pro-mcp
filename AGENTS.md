# Fender Studio Pro MCP

## Start a chat

- Read the latest user request and the relevant files in this checkout. Start a new feature from current `origin/main` in an isolated branch or worktree.
- For plugin implementation or validation, read the relevant sections of [the MCP development guide](docs/MCP_DEVELOPMENT.md). Use the installed pstack workflow and Codex adapter for substantive engineering work.
- An explicit resume such as `진행 재개해` continues the documented scope. Follow [the resume guide](docs/RESUME.md) to the shared handoff branch. Read `status` and `resume_document` in its `docs/RESUME_STATE.json`, then read the sections you need. Carry forward existing authorization and completed evidence. Check current GitHub, host, and hardware state.

## Implement and verify

- Keep the MCP server in native Rust. Keep JavaScript inside the DAW bridge and EditTask. Expose finite typed operations and preserve upstream MIT notices.
- Serialize DAW access across processes. Check the request ID, bridge session, and active document. Bind plugin changes to the exact native instance. Treat lost replies after dispatch as unknown. Read fresh state before deciding how to recover.
- Run authorized live checks in a named scratch song with one Studio Pro process and one MCP owner. Verify the app, plugin, build, and relevant UI state. Preserve user songs and settings, and restore the normal installation after diagnostics.
- Test on macOS arm64 first. Check official plugin distributions and existing MCP or SDK paths before using REA and Ghidra for unresolved behavior. Keep raw songs, audio, app logs, local paths, and proprietary resources out of Git. Public evidence belongs in `docs/evidence/`.
- Record the exact version and scope of each verified result. A setter receipt, independent readback, saved state, and physical audio or MIDI each have their own evidence.

## Report and ship

- Report in Korean. Use Korean or English for internal notes. After each unit, report the completed checklist, measured elapsed time, remaining count, next-unit and overall estimates, and time until the current session cutoff if one applies. Call out hardware dependencies and changes in scope.
- Use neutral branch names and PR titles without `codex`. Open a separate PR as soon as an independently usable unit is verified. Merge it once both required checks pass for the exact head against current `main`. Close superseded drafts and retain branches needed for research or handoff.
- At a pause, record the last verified unit, next action, open PRs, and cleanup in the dated handoff. Update the state file's handoff pointer and push the checkpoint.
