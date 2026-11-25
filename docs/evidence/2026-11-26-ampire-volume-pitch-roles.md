# Ampire global Volume and Pitch role investigation

R433 inspected the arm64 Ampire image shipped with Studio Pro 8.1.1 Build 113103. The extracted slice matched the R425 image, UUID `E83B8167-CF19-3B5B-A5C6-E7EBD743AB31`, SHA-256 `a6f391f97b20f872ef2614eca7af8cd8d68c541bedc34ecba2fd57cbd0379027`. R433 cross-checked Mach-O fixups and LLVM instructions against the original bytes and ran an independent Craft Anything analysis. The verifier checked seven role factories, 14 role methods, eight alias slots and 15 additional instruction anchors. Twelve Craft procedures returned, including one unstructured result.

| Evidence | Observation |
| --- | --- |
| Prior live catalog, R360 | The global Volume (`0xE003`) and Pitch (`0xE005`) rows returned raw zero and empty display text for a vacant slot and all 15 installed stomp models in slot 2. The catalog marked both enabled and writable. No write was dispatched to either row. |
| Native role providers | The seven directly observed `IFac`/`ParR` providers returned one Wah role and six Speed roles at their declared local indices. None of those methods emitted a Volume or Pitch role. The Wah role points to the local pedal control, and the PAE Chorus 1 Speed role points to the local vibrato rate. Earlier R359 and R357 live readback and restoration established those pairs. |
| FAT Volume | The local FAT Volume descriptor, setter, and getter use local index 3 and share the same model value. Its factory returns without advertising an `IFac`/`ParR` role. The editor and remote page expose this local control. |
| Generic CCL alias behavior | The registered `AliasParam` forwards reads and writes when its original pointer is present. With a null original, the value reads return zero, the formatter returns an empty string, and writes do not forward. The generic object reports disabled in that case. |
| Editor and remote controls | The installed skin’s seven XML members and 23 RemotePages XML descriptions supplied no global Volume or Pitch binding. The same inspection found positive bindings for local Volume, Wah and Chorus controls. No separate MIDI assignment or processor route was established for the two global rows. |

The generic null behavior is consistent with the observed zero and blank values. The catalog’s enabled flag and the lack of a live wrapper class or pointer read leave the actual alias binding unresolved. The diagnostic’s self-reference was also present for the proven Wah and Speed pairs, so it cannot identify the state of a live alias. The factory scan covers the directly observed encoding on this image. Other host dispatch paths remain outside its scope.

The current evidence supports documenting both global rows as unverified for mutation and audio. A future live trial would first need an actual provider, an exact child parameter and independent movement of the pair. This 10-minute-29-second investigation used existing live records and performed no new DAW operations.
