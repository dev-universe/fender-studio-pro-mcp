from pathlib import Path
import re,sys
s=Path(sys.argv[1]).read_text();functions=dict(re.findall(r"^_menu_([^:]+):[^\n]*\n(.*?)(?=^\s*; -- End function)",s,re.M|re.S))
expected={"construct":24,"build":32,"class":24,"count":64,"item":72,"attr":24,"sub":48,"create":24}
for name,offset in expected.items():
 found=re.findall(r"ldr\s+x\d+, \[x\d+, #(\d+)\]",functions[name])
 if found != [str(offset)]:raise SystemExit(f"menu ABI mismatch {name}: {found}, expected {offset}")
print("Checked8 menu/factory virtual call offsets and compiler layout assertions")
