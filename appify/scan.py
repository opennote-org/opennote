import subprocess
import threading
from collections import defaultdict
from concurrent.futures import Executor
from concurrent.futures.thread import ThreadPoolExecutor


class RecursiveLibraryScanner:
    def __init__(self, executor: Executor, scan_private: bool):
        self.executor = executor
        self.libraries = defaultdict(set)
        self.scanned = set()
        self.scan_private = scan_private
        self.jobs = []
        self.all_done = threading.Event()

    def _check(self, job):
        if all(j.done() for j in self.jobs):
            self.all_done.set()

    def _enqueue(self, target):
        job = self.executor.submit(self._scan, target)
        job.add_done_callback(self._check)
        self.jobs.append(job)

    def _scan(self, target):
        # print("scanning", target, file=sys.stderr)
        self.scanned.add(target)
        for lib in scan_libraries(target):
            self.libraries[target].add(lib)
            if lib not in self.scanned:
                is_private = smells_private(lib)
                if (is_private and self.scan_private) or not is_private:
                    self._enqueue(lib)

    def scan(self, target):
        self._enqueue(target)
        self.all_done.wait()
        return self.libraries


def scan_libraries_recursive(initial_target, scan_private=True):
    with ThreadPoolExecutor() as executor:
        rls = RecursiveLibraryScanner(executor, scan_private=scan_private)
        return rls.scan(initial_target)


def scan_libraries(target):
    in_load_dylib = False
    libraries = set()
    for line in subprocess.check_output(
        ["otool", "-l", target], encoding="utf-8"
    ).splitlines():
        line = line.strip()
        if line == "cmd LC_LOAD_DYLIB":
            in_load_dylib = True
        if in_load_dylib and line.startswith("name "):
            words = line.split()
            lib = words[1]
            libraries.add(lib)
            in_load_dylib = False
    return libraries


def smells_private(lib):
    if lib.startswith("/System/Library"):
        return True
    if lib.startswith("/usr/lib/"):
        return True
    if lib.startswith("/usr/local/lib/"):
        return True
    return False


def filter_private(scanned_libraries):
    public_libraries = {
        target: {lib for lib in libraries if not smells_private(lib)}
        for (target, libraries) in scanned_libraries.items()
        if not smells_private(target)
    }
    return public_libraries
