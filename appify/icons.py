import subprocess
import tempfile
from multiprocessing.pool import ThreadPool

sips_templates = [
    ["16", "icon_16x16.png"],
    ["32", "icon_16x16@2x.png"],
    ["32", "icon_32x32.png"],
    ["64", "icon_32x32@2x.png"],
    ["128", "icon_128x128.png"],
    ["256", "icon_128x128@2x.png"],
    ["256", "icon_256x256.png"],
    ["512", "icon_256x256@2x.png"],
    ["512", "icon_512x512.png"],
]


def generate_icns(target_filename: str, source_png: str) -> str:
    with tempfile.TemporaryDirectory(suffix=".iconset") as iconset_dir:
        sips_commands = [
            ["sips", "-z", size, size, source_png, "-o", f"{iconset_dir}/{target}"]
            for (size, target) in sips_templates
        ]
        with ThreadPool() as p:
            list(
                p.imap_unordered(
                    lambda cmd: subprocess.check_call(cmd, stdout=subprocess.DEVNULL),
                    sips_commands,
                )
            )
        subprocess.check_call(
            ["iconutil", "-c", "icns", "-o", target_filename, iconset_dir,]
        )
    return target_filename
