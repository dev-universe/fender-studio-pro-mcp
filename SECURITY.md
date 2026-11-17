# Report a security vulnerability

Use [GitHub private vulnerability reporting](https://github.com/dev-universe/fender-studio-pro-mcp/security/advisories/new).
Do not publish a vulnerability in an issue, pull request, discussion, or raw log.
If the private form is unavailable, do not post the details publicly.

Include the affected commit, the relevant MCP tool or bridge operation, a
minimal reproduction with synthetic data, and the observed impact. Remove
credentials, personal paths, songs, audio, and proprietary app resources.

The maintained security target is the current `main` branch. Older commits and
unreviewed host versions have no separate security support commitment.

Reports can cover guard bypasses, cross-process access, unintended DAW changes,
path traversal, data disclosure, or arbitrary code execution. A successful build
or offline test does not establish safety in an unverified DAW version. See
[native integration](native/README.md) for the validated runtime and ABI limits.
