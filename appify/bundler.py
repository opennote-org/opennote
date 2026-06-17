import os
import plistlib
import shutil
import stat
import subprocess
from pathlib import Path
from typing import List, Optional

from icons import generate_icns
from scan import scan_libraries, smells_private


def generate_bundle(
    *,
    bundle_dir: str,
    original_executable: str,
    required_libraries: List[str],
    icon_png: Optional[str] = None,
    bundle_identifier: Optional[str] = None,
):
    executable_name = os.path.basename(original_executable)
    plist_content = {
        "CFBundleDevelopmentRegion": "English",
        "CFBundleDisplayName": executable_name,
        "CFBundleExecutable": executable_name,
        "CFBundleIconFile": executable_name,
        "CFBundleIdentifier": (
            bundle_identifier or f"appify.{executable_name}"
        ),
        "CFBundleInfoDictionaryVersion": "6.0",
        "CFBundleName": executable_name,
        "CFBundlePackageType": "APPL",
        "CFBundleShortVersionString": "0.0",
        "CFBundleVersion": "0.0",
        "LSHasLocalizedDisplayName": False,
        "NSAppleScriptEnabled": False,
        "NSHumanReadableCopyright": "Copyright not specified",
        "NSHighResolutionCapable": True,
    }
    contents_dir = Path(bundle_dir) / "Contents"
    bin_dir = contents_dir / "MacOS"
    res_dir = contents_dir / "Resources"
    lib_dir = bin_dir
    for dir in (bin_dir, res_dir, lib_dir):
        if not dir.exists():
            dir.mkdir(parents=True)
    if icon_png:
        icns_name = f"{executable_name}.icns"
        print("Generating ICNS file...")
        generate_icns(str(res_dir / icns_name), icon_png)
        plist_content["CFBundleIconFile"] = icns_name
    (contents_dir / "PkgInfo").write_bytes(b"APPLPMDS")
    with (contents_dir / "Info.plist").open("w+b") as plist_fp:
        plistlib.dump(plist_content, plist_fp)
    executable_path = bin_dir / executable_name
    print(f"Copying {original_executable}")
    shutil.copy(original_executable, executable_path)
    os.chmod(executable_path, 0o744)  # RWXR--R--

    required_fixups = []
    fixup_targets = {str(executable_path)}
    for lib in required_libraries:
        libname = os.path.basename(lib)
        target = lib_dir / libname
        if target.exists():
            print(f"Copying {lib} – replacing existing {target}")
            target.unlink()
        else:
            print(f"Copying {lib}")
        shutil.copy(lib, target)
        os.chmod(target, os.stat(target).st_mode | stat.S_IWUSR)
        required_fixups.extend(
            [
                "-change",
                str(lib),
                f"@rpath/{libname}",
            ]
        )
        fixup_targets.add(str(target))
    print(f"Running fixups...")
    required_fixups.extend(["-add_rpath", "@executable_path"])
    for fixup_target in fixup_targets:
        command_line = ["install_name_tool"] + required_fixups + [fixup_target]
        subprocess.check_call(command_line)
    print(f"Verifying fixups...")
    for fixup_target in fixup_targets:
        bad_deps = {
            lib
            for lib in scan_libraries(fixup_target)
            if not (smells_private(lib) or lib.startswith("@rpath"))
        }
        if bad_deps:
            print(f"{fixup_target} may still have bad deps: {bad_deps}")
