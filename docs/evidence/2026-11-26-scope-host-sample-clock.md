# Scope and the Studio Pro host sample clock

R446 examined the installed Studio Pro 8.1.1 Build 113103 arm64 executable, its CoreAudio service, the bundled vendor script SDK, and the CCL 5.1.2 interfaces. The main-executable slice matched the prior exact image, UUID `6B547378-B3BB-322A-87C9-901493FD2BED`, SHA-256 `c6b326d8dfac4d468f8730508abcf8687997c6cd96a977a560600fda785a2e6e`.

The vendor SDK reaches the AudioEngine object and reads its master-device name. In the same parameter list, the executable publishes a numeric `sampleRate` from the attached device’s getter, with a 44,100 Hz fallback for a missing device or nonpositive result. The candidate read path is the engine’s named numeric parameter. Its value has not yet been independently read from a live MCP session. The 48.0 kHz value displayed by Studio Pro in R443 is a separate UI observation.

The installed CoreAudio service reads input sample time and host time on its device I/O callback and transfers them to a private receiver with the device format. The public CCL media clock supplies monotonic seconds. The observer queue and the existing MCP bridge use main-thread delivery. The inspected paths do not publish a coherent trigger latch, collected count, or sample time for an individual Scope capture.

The nine-minute static audit gives a bounded engine-reported rate candidate. The remaining Scope check needs one producer-owned observation tying the exact instance and its process format to a capture generation, frame clock, and armed or completed state. No new DAW operation, observer registration, or runtime feature was exercised.
