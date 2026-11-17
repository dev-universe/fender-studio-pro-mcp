"""Compare exactly the fixed read-only QI registry with the pinned SDK declarations."""
from pathlib import Path
import re
import sys

if len(sys.argv) != 2:
    raise SystemExit("Usage: check_declared_iids.py /path/to/ccl-framework-5.1.2")
root = Path(sys.argv[1])
source = Path(__file__).parents[1].joinpath("src/lib.rs").read_text()
registry = {
    "PRESET_COLLECTOR_IID": ("IPresetCollector", "ccl/public/app/ipreset.h"),
    "PRESET_MEDIATOR_IID": ("IPresetMediator", "ccl/public/app/ipreset.h"),
    "STORABLE_IID": ("IStorable", "ccl/public/storage/istorage.h"),
}
for constant, (interface, header) in registry.items():
    declaration = re.search(r"DEFINE_IID\s*\(\s*" + interface + r"\s*,([^)]*)\)",
                            root.joinpath(header).read_text())
    rust = re.search(r"const " + constant + r": Uid = Uid \{(.*?)\};", source, re.S)
    if declaration is None or rust is None:
        raise SystemExit(f"Missing reviewed fixed IID declaration: {interface}")
    values = lambda text: [int(value, 16) for value in re.findall(r"0x[0-9a-fA-F]+", text)]
    expected, actual = values(declaration[1]), values(rust[1])
    if len(expected) != 11 or actual != expected:
        raise SystemExit(f"Fixed observation IID differs from pinned SDK: {interface}")
fixed = re.search(r"const DECLARED_INTERFACE_IIDS: \[Uid; 3\]\s*=\s*\[([^]]+)\]", source, re.S)
if fixed is None or re.findall(r"[A-Z_]+_IID", fixed[1]) != list(registry):
    raise SystemExit("Fixed observation registry differs from the three reviewed interfaces")
result_header = root.joinpath("ccl/public/base/iunknown.h").read_text()
for constant, sdk_name in {"QI_RESULT_OK": "kResultOk",
                           "QI_RESULT_NO_INTERFACE": "kResultNoInterface"}.items():
    sdk = re.search(r"constexpr tresult " + sdk_name + r"\s*=\s*static_cast<tresult>\s*\(\s*(0x[0-9a-fA-F]+)L?\s*\)", result_header)
    rust = re.search(r"const " + constant + r": i32 = (0x[0-9a-fA-F]+)(?:u32 as i32)?;", source)
    if sdk is None or rust is None or int(sdk[1], 16) != int(rust[1], 16):
        raise SystemExit(f"Fixed QI status differs from pinned SDK: {sdk_name}")
print("PASS: three declared-interface IIDs and both QI result constants match public CCL5.1.2 headers")
