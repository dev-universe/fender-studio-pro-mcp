"""Check that the SDK guard rejects a newer official ABI without loading it."""
from pathlib import Path
import subprocess
import sys
import tempfile

if len(sys.argv) != 2:
    raise SystemExit("Usage: regression.py /path/to/separate-ccl-framework-5.2")
newer = Path(sys.argv[1])
if not newer.exists():
    raise SystemExit("Newer SDK fixture missing; provide its separately obtained path")
result = subprocess.run(
    [sys.executable, str(Path(__file__).with_name("verify_sdk.py")), str(newer)],
    capture_output=True, text=True,
)
if result.returncode == 0 or "REFUSED CCL SDK" not in result.stderr:
    raise SystemExit(f"Expected version mismatch rejection, got {result}")
with tempfile.TemporaryDirectory(prefix="parameter-abi-regression-") as directory:
    assembly = Path(directory) / "newer.s"
    subprocess.run([
        "clang++", "-std=c++20", "-arch", "arm64", f"-I{newer}",
        "-Werror", "-Wno-return-type-c-linkage", "-O1", "-S",
        str(Path(__file__).with_name("abi_calls.cpp")), "-o", str(assembly),
    ], check=True, capture_output=True, text=True)
    offsets = subprocess.run([
        sys.executable, str(Path(__file__).with_name("check_abi.py")), str(assembly),
    ], capture_output=True, text=True)
    if offsets.returncode == 0 or "ABI offset mismatch" not in offsets.stderr:
        raise SystemExit(f"Expected independently detected call map mismatch, got {offsets}")
print("PASS: official CCL5.2 is refused by SDK pin and independently fails compiled virtual call map")
