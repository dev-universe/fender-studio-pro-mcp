# Fender Studio Pro MCP

A Rust MCP server for Fender Studio Pro. macOS is the first target.

The server will combine native CoreMIDI with a small in-DAW JavaScript bridge
and EditTask extension. The implementation starts with real Studio Pro 8.1.1
validation. Built-in effects and instruments are a required control target.

Development is in progress. Source inspection and the implementation plan are
available. Runtime support is reported only after a real DAW check passes.

- [Implementation plan](docs/IMPLEMENTATION_PLAN.md)
- [Reference project research](docs/RESEARCH.md)

This independent project is not affiliated with Fender or PreSonus. Fender
Studio Pro and Studio One are trademarks of their respective owners.

