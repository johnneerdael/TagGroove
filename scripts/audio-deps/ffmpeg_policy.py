"""Strict replay of reviewed FFmpeg patches; no shell, fuzz or archive execution."""
import hashlib
from pathlib import PurePosixPath
import re

TARGETS = {'configure', 'libavcodec/aarch64/Makefile', 'libavcodec/ac3dsp.c',
           'libavutil/aarch64/Makefile', 'libavutil/crc.c'}
GPL_SOURCES = {'libavcodec/aarch64/ac3dsp_init_aarch64.c',
               'libavcodec/aarch64/ac3dsp_neon.S', 'libavutil/aarch64/crc.S'}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def validate_patches(patches):
    targets, files = set(), set()
    for entry in patches:
        target, filename = entry.get('target'), entry.get('file')
        require(entry.get('component') == 'ffmpeg' and target in TARGETS,
                'Unexpected native patch target')
        require(isinstance(filename, str) and filename.startswith('patches/')
                and PurePosixPath(filename).as_posix() == filename
                and '..' not in PurePosixPath(filename).parts and '\\' not in filename,
                'Unsafe native patch file')
        require(target not in targets and filename not in files, 'Duplicate native patch target or file')
        targets.add(target)
        files.add(filename)
        for field in ['sha256', 'original_sha256', 'patched_sha256']:
            require(re.fullmatch('[a-f0-9]{64}', entry.get(field, '')) is not None,
                    'Invalid native patch hash')


def patched_path(target):
    require(target in TARGETS, 'Unexpected native patch target')
    return 'ffmpeg-patched-configure' if target == 'configure' else 'ffmpeg-patched/' + target


def apply_patch(entry, original, diff):
    validate_patches([entry])
    require(sha(original) == entry['original_sha256'], 'Patch original upstream file hash differs')
    require(sha(diff) == entry['sha256'], 'Native patch source hash differs')
    lines = diff.decode('utf-8').splitlines(keepends=True)
    target = entry['target']
    require(len(lines) >= 3 and lines[:2] == ['--- a/' + target + '\n', '+++ b/' + target + '\n'],
            'Native patch target headers differ')
    source = original.decode('utf-8').splitlines(keepends=True)
    result, cursor, index, hunks = [], 0, 2, 0
    while index < len(lines):
        match = re.fullmatch(r'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@\n', lines[index])
        require(match is not None, 'Invalid native patch hunk')
        old_start, old_count = int(match[1]), int(match[2] or 1)
        new_start, new_count = int(match[3]), int(match[4] or 1)
        start = old_start - 1 if old_count else old_start
        require(cursor <= start <= len(source), 'Native patch hunk position differs')
        result.extend(source[cursor:start])
        cursor = start
        require(len(result) == (new_start - 1 if new_count else new_start), 'Native patch new position differs')
        index += 1
        removed = added = 0
        while index < len(lines) and not lines[index].startswith('@@ '):
            line = lines[index]
            require(line[:1] in {' ', '+', '-'} and line.endswith('\n'), 'Invalid native patch line')
            if line[0] in ' -':
                require(cursor < len(source) and source[cursor] == line[1:], 'Native patch context differs')
                cursor += 1
                removed += 1
            if line[0] in ' +':
                result.append(line[1:])
                added += 1
            index += 1
        require(removed == old_count and added == new_count, 'Native patch hunk counts differ')
        hunks += 1
    require(hunks > 0, 'Empty native patch')
    result.extend(source[cursor:])
    patched = ''.join(result).encode('utf-8')
    require(sha(patched) == entry['patched_sha256'], 'Applied native patch output hash differs')
    return patched


def verify_no_gpl_sources(sources):
    require(not GPL_SOURCES.intersection(sources), 'Forbidden GPL source in no-GPL compiler closure')
