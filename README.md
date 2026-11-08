# Fender Studio Pro MCP

A native Rust MCP server that lets an LLM read and control Fender Studio Pro through typed tools. The server connects to a JavaScript bridge and EditTasks inside the DAW through a local mailbox and CoreMIDI.

Actual DAW checks use **Fender Studio Pro 8.1.1 Build 113103 on macOS Apple Silicon**. The bridge, MIDI note creation and editing, 55 representative built-in parameter roundtrips and 3 additional bypass roundtrips have passed live checks. Complete control of every built-in plugin remains unfinished. Read the [validation report](docs/VALIDATION.md) and [plugin matrix](docs/BUILTIN_PLUGINS.md) for the verified scope and open paths.

The tools cover session and transport state, tracks and mixer channels, MIDI parts and notes, supported plugin parameters, and finite editing operations. Saved-song inspection and WAV analysis also run without a DAW connection. Current surface mappings cover at most 256 mixer channels, 16 insert slots and 8 sends per channel; note-edit batches cover at most 128 parts. Windows, Intel Mac, and Linux DAW integration have no actual-environment validation.

Build with Rust 1.88 or newer and Cargo. The installed server runs as a native binary. Node.js and Python are development and validation tools. The server does not launch either runtime.

Follow these steps for the first connection.

1. Clone, build, and install the bridge and EditTasks.

   ```sh
   git clone https://github.com/dev-universe/fender-studio-pro-mcp.git
   cd fender-studio-pro-mcp
   cargo build --locked --release
   ./target/release/fender-studio-pro-mcp install
   ```

   The default DAW profile is `~/Library/Application Support/Fender/Studio Pro 8`. For a different profile, pass `--profile /path/to/profile` to each command. Installation adds the managed device and extension assets and preserves unrelated settings.

2. Start the MCP server through your client using the configuration below. For initial device setup without an MCP client, keep this command running in a terminal.

   ```sh
   ./target/release/fender-studio-pro-mcp hold
   ```

   The `hold` command prints `"ready": true` and publishes the virtual MIDI source. Keep the owner process running while you configure the DAW.

3. Quit and restart Studio Pro after installation, with the server or `hold` process already running.

4. Open **External Devices**, add a **Control Surface**, and select these settings.

   | Setting | Value |
   | --- | --- |
   | Vendor | `fender-studio-pro-mcp` |
   | Device | `Fender MCP Bridge` |
   | Receive From | `fender-studio-pro-mcp` |
   | Send To | `None` |

5. Open a session. Use a named disposable scratch session for the first editing checks, with playback and recording stopped. The validated request path requires an open session. The Home screen can time out.

6. To check the installation and current connection, first disconnect the MCP client or stop `hold` with Ctrl-C. Then run the standalone diagnostic.

   ```sh
   ./target/release/fender-studio-pro-mcp doctor --live
   ```

   Look for `"installation_current": true` and `"live_verified": true`. A stored `status_file` alone reports a previous bridge load. It does not verify the current connection.

7. Start your MCP client again. Only one server, `hold`, live diagnostic, or CLI controller can own the same profile at a time. A second owner returns `controller_busy`.

Use an absolute binary path in a client that supports stdio MCP servers. Adapt this generic configuration to your client's settings format.

```json
{
  "mcpServers": {
    "fender-studio-pro": {
      "command": "/path/to/fender-studio-pro-mcp/target/release/fender-studio-pro-mcp",
      "args": ["serve"]
    }
  }
}
```

The client starts `serve` and exchanges MCP JSON-RPC through stdin and stdout. The server writes diagnostics to stderr. Keep the server alive during the DAW session. If the input source disconnects after a server restart, reselect **Receive From** with the server running. Restarting the DAW with the owner already running is the first-connection sequence above. Reconnection across every launch order remains unverified.

Ask the LLM to call `host_status` before editing. Mutations require its current `document` token as `expected_document`. Reopening or switching a document invalidates earlier editing intent. Call `operations_list` to inspect the finite operation schema and tool capabilities.

For standalone CLI calls, stop any other owner first. Read the current context.

```sh
./target/release/fender-studio-pro-mcp call context
```

For a scratch channel named `Scratch` with Mixtool in insert slot 0, read the parameter before setting it. Replace the names and document token with the values from your own scratch session. Insert slots start at zero, and normalized values range from 0 to 1.

```sh
./target/release/fender-studio-pro-mcp call pluginParams \
  '{"kind":"effect","channel":"Scratch","slot":0,"plugin":"Mixtool","names":["gain"]}'

./target/release/fender-studio-pro-mcp call setPluginParam \
  '{"kind":"effect","channel":"Scratch","slot":0,"plugin":"Mixtool","param":"gain","normalized":0.5}' \
  --expected-document 'DOCUMENT_TOKEN_FROM_CONTEXT'
```

Plugin writes require the expected plugin class or instance name. Parameter discovery uses partial metadata candidates. A missing candidate does not prove that the plugin lacks the control. Class matching also cannot distinguish replacement by another instance of the same class in the same slot.

Read each outcome receipt. `applied_verified` includes operation-specific readback. `applied_unverified` acknowledges an edit without complete verification. `partially_applied` preserves successful earlier work. `outcome_unknown` can mean the edit happened before a timeout. Inspect the actual state before deciding whether to repeat an operation.

These commands inspect local files without connecting to the DAW or taking its controller lock.

```sh
./target/release/fender-studio-pro-mcp local-catalog --plugin 'Pro EQ'
./target/release/fender-studio-pro-mcp song-read /path/to/Scratch.song
./target/release/fender-studio-pro-mcp audio-analyze /path/to/render.wav
```

`local-catalog` reads candidate parameter names from the installed app's metadata. `song-read` reports the last saved archive, including partial plugin snapshots, rather than unsaved live edits. `audio-analyze` reports WAV loudness, peak, RMS, and stereo measurements with explicit format and measurement limits. Dedicated DAW capture and export workflows remain unfinished.

After a source update, rebuild, stop the active owner, run `install`, and restart Studio Pro. The installer refuses to overwrite modified managed assets without `install --force`. Back up your changes before using that option. To remove unchanged managed assets, stop the owner and run `uninstall`. Locally modified managed files are preserved.

Use these documents for details.

- [Validation report and known limits](docs/VALIDATION.md).
- [Built-in plugin matrix](docs/BUILTIN_PLUGINS.md).
- [Implementation plan](docs/IMPLEMENTATION_PLAN.md).
- [Architecture](docs/ARCHITECTURE.md).
- [Reference project research](docs/RESEARCH.md).
- [Local checks and CI status](ci/README.md).

This independent project is not affiliated with Fender or PreSonus. Fender Studio Pro and Studio One are trademarks of their respective owners. The project uses the [MIT license](LICENSE). Copied upstream assets retain their notices in [third-party notices](THIRD_PARTY_NOTICES.md).

