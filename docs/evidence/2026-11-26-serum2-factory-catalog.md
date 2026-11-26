# Serum 2 Factory preset catalog

R498 ran the Rust MCP server over stdio on macOS arm64 with Studio Pro
closed. An independent file reader matched all 626 reported entries in
the installed Factory library, including 624 Serum 2 instrument
presets and two Serum 2 FX presets. The comparison checked the order,
variant, name, relative path, product version and schema. Five pages
shared the same catalog digest. Separate calls confirmed variant
filtering, case-insensitive search and an empty search result.

The installed files reported versions 2.0.11–2.0.15 with schemas 3–6.
The tool reads the fixed envelope, metadata and frame header. Catalog
entries give the path for the Serum editor. Processor-only 2.1.5 preset
application keeps its existing version and schema requirements.

Three invalid page requests were rejected. Every preset file matched
its original SHA-256 after the calls, and the isolated MCP server exited
normally. The public test took 2.974 seconds.
