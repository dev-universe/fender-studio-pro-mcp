# Mai Tai Character in Studio Pro 8.1.1

R337 used one owned scratch and the exact Mai Tai instrument of class
`B625F134-4485-4A50-A3C8-C9CF0C5495E1` in Studio Pro 8.1.1 build 113103 on
macOS arm64. The selected track was unarmed and unmonitored. The transport
was stopped and Main was at zero.

| Action | Visible result | Two independent typed reads |
| --- | --- | --- |
| Open the stock editor | `GrandClass`, Sound and Amount appear in the Character panel. | `character1.presets=0`, `character1.type=Blend`, `character1.editchar=0`. |
| Set the exact `character1.editchar` toggle | The same stock panel remains visible with no additional editor. | The toggle at tag 110 is the only changed row, 0 → 1. |
| Select `CharacterSaw` using the stock menu | The Character panel selects `CharacterSaw`. | Only `character1.presets` changes, 0 → 3. `character1.type` still reads `Blend` and `editchar` stays 1. |
| Select `GrandClass`, then reset the toggle | The stock panel displays `GrandClass` again. | The selected preset and then the toggle return to 0. All 283 writable rows and mixer observations match the original. |

The bundled Mai Tai skin binds the stock menu to `character1.presets` and
the two knobs to `eqcolour` and `shapemix`. The shipped arm64 `instruments`
image has Mach-O UUID `EFDFFE66-D1E7-35E6-A0E2-FDD9C92B2C7E`. REA/Ghidra
resolved the two `CharacterEdit` lookup entry points and Craft Anything
Deep verified the false value stored by the matching branch. This agrees
with the observed typed toggle and the stock editor in the tested build.
The menu observation covers the GrandClass and CharacterSaw selections.
The DSP `type` row by itself does not identify the selected public model.
[Fender Mai Tai manual](https://fenderstudiopromanual.fender.com/en/Content/Built-In_Instruments_Topics/Mai_Tai.htm)

The trial recorded 96 requests across three consecutive owners and the host
was open for 200.299 seconds. A schema rejection happened before the initial
editor dispatch. A later guard required a `type` change, and that attempt
stopped when the typed reads showed only the expected visible preset had
changed. Restoration used a fresh guarded owner in the same document and
instance. The scratch closed without saving and its original bytes matched.
The 18 managed files and ten changed profile files were restored.
