#!/usr/bin/env python3
"""Read authenticated Debian package members without installation or target execution."""
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import threading


_spec = importlib.util.spec_from_file_location('linux_source_material', Path(__file__).with_name('linux-source-material.py'))
material = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(material)
require, sha = material.require, material.sha


def member_path(value, base=(), link=False):
    require(isinstance(value, str) and value and '\0' not in value and '\\' not in value,
            'Invalid binary package member path')
    require(link or not value.startswith('/'), 'Absolute binary package member name')
    parts = [] if value.startswith('/') else list(base)
    for part in value.split('/'):
        if part in ('', '.'):
            continue
        if part == '..':
            require(link and parts, 'Binary package path escapes its archive')
            parts.pop()
        else:
            parts.append(part)
    return '/'.join(parts)


class BoundedRead:
    def __init__(self, stream, limit):
        self.stream, self.used, self.limit = stream, 0, limit

    def read(self, size=-1):
        remaining = self.limit - self.used
        raw = self.stream.read(min(size if size >= 0 else remaining + 1, remaining + 1))
        self.used += len(raw)
        require(self.used <= self.limit, 'Binary package archive exceeds scan bounds')
        return raw


def scan_payload(tool, package, wanted, control=False):
    index, bodies = {}, {}
    option = '--ctrl-tarfile' if control else '--fsys-tarfile'
    stream_limit = 16 * 1024 * 1024 if control else 2 * 1024**3
    member_limit = 4 * 1024 * 1024 if control else 512 * 1024 * 1024
    with tempfile.TemporaryFile() as errors:
        child = subprocess.Popen([tool, option, str(package)], stdout=subprocess.PIPE, stderr=errors)
        timer = threading.Timer(60, lambda: child.kill() if child.poll() is None else None)
        timer.daemon = True
        timer.start()
        try:
            with tarfile.open(fileobj=BoundedRead(child.stdout, stream_limit), mode='r|', ignore_zeros=True) as archive:
                for entry in archive:
                    path = member_path(entry.name)
                    if not path:
                        require(entry.isdir(), 'Binary package root is not a directory')
                        continue
                    require(path not in index and len(index) < 100000, 'Duplicate or excessive binary package members')
                    require(0 <= entry.size <= member_limit, 'Binary package member exceeds bounds')
                    kind = ('file' if entry.isfile() else 'directory' if entry.isdir() else
                            'symlink' if entry.issym() else 'hardlink' if entry.islnk() else 'other')
                    index[path] = {'kind': kind, 'size': entry.size, 'link': entry.linkname}
                    if path in wanted:
                        require(kind == 'file', 'Expected a regular binary package member')
                        raw = archive.extractfile(entry).read()
                        require(len(raw) == entry.size, 'Truncated binary package member')
                        bodies[path] = raw
            require(child.wait(timeout=60) == 0, 'Unable to read binary package payload')
        except tarfile.TarError as error:
            raise ValueError('Invalid binary package payload') from error
        finally:
            timer.cancel()
            child.stdout.close()
            if child.poll() is None:
                child.terminate()
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                child.kill();child.wait()
    return index, bodies


def resolve_member(index, requested):
    current, seen = requested, set()
    for _ in range(64):
        require(current not in seen and current in index, 'Missing or cyclic binary package alias')
        seen.add(current)
        entry = index[current]
        if entry['kind'] == 'file':
            return current
        require(entry['kind'] in {'symlink', 'hardlink'}, 'Unsupported binary package member kind')
        base = current.split('/')[:-1] if entry['kind'] == 'symlink' else ()
        current = member_path(entry['link'], base=base, link=True)
    raise ValueError('Binary package alias depth exceeds bounds')


def package_members(raw, selected, requested):
    require(len(raw) == selected['size'] and sha(raw) == selected['sha256'],
            'Binary archive differs from authenticated repository selection')
    names = list(requested)
    require(names and len(names) == len(set(names)), 'Missing or duplicate requested binary members')
    require(all(member_path(name) == name for name in names), 'Requested binary path is not canonical')
    tool = shutil.which('dpkg-deb')
    require(tool is not None, 'Binary package verification requires dpkg-deb')
    with tempfile.TemporaryDirectory(prefix='taggroove-package-members-') as folder:
        package = Path(folder)/'input.deb'
        package.write_bytes(raw)
        _control_index, controls = scan_payload(tool, package, {'control'}, control=True)
        require(controls.get('control'), 'Missing binary package control identity')
        records = material.paragraphs(controls['control'])
        require(len(records) == 1, 'Expected one binary package control record')
        control = records[0]
        require(all(control.get(field) == selected[field] for field in ['package', 'version', 'architecture'])
                and material.binary_source(control) == (selected['source_package'], selected['source_version']),
                'Binary control identity differs from authenticated repository selection')
        index, _ = scan_payload(tool, package, set())
        resolved = {name: resolve_member(index, name) for name in names}
        final_index, bodies = scan_payload(tool, package, set(resolved.values()))
        require(final_index == index, 'Binary package member set changed between scans')
    return {name: {'path': path, 'bytes': bodies[path]} for name, path in resolved.items()}
