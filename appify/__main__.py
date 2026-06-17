import argparse

from bundler import generate_bundle
from scan import filter_private, scan_libraries_recursive
from utils import walk_graph


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--executable", required=True)
    ap.add_argument("--bundle", required=True)
    ap.add_argument("--bundle-identifier")
    ap.add_argument("--icon-png")
    args = ap.parse_args()

    original_executable = args.executable
    print(f"Scanning library dependencies for {original_executable}...")
    scanned_libraries = scan_libraries_recursive(
        original_executable, scan_private=False
    )
    public_libraries = filter_private(scanned_libraries)
    required_libraries = list(walk_graph(public_libraries, original_executable))
    required_libraries.remove(original_executable)
    print(f"{len(required_libraries)} required libraries.")
    assert args.bundle.endswith(".app")
    generate_bundle(
        bundle_dir=args.bundle,
        original_executable=original_executable,
        required_libraries=required_libraries,
        icon_png=args.icon_png,
        bundle_identifier=args.bundle_identifier,
    )


if __name__ == "__main__":
    main()
