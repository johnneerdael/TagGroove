#!/usr/bin/env python3
"""Read generated fixtures from the dedicated Android UAT device; never drive UI."""
import argparse
import hashlib
import io
import json
import re
import shutil
import subprocess
import tarfile
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path('/tmp/opentag-user-test/receipts/android-documents')
SERIAL = 'emulator-5554'
AVD = 'OpenTag_Document_Test_API_34'
ADB = shutil.which('adb') or '/Users/jneerdael/Library/Android/sdk/platform-tools/adb'
FFPROBE = shutil.which('ffprobe') or '/opt/homebrew/bin/ffprobe'
FFMPEG = shutil.which('ffmpeg') or '/opt/homebrew/bin/ffmpeg'


def command(args, required=True):
    result = subprocess.run(args, capture_output=True)
    if required and result.returncode:
        raise RuntimeError(f'{args}: {result.stderr.decode(errors="replace")}')
    return result


def collect(package, directory, output):
    result = command([ADB, '-s', SERIAL, 'exec-out', 'run-as', package,
                      'tar', '-C', directory, '-cf', '-', '.'])
    with tarfile.open(fileobj=io.BytesIO(result.stdout)) as archive:
        for member in archive:
            if member.isdir():
                continue
            relative = Path(member.name)
            if not member.isfile() or relative.is_absolute() or '..' in relative.parts:
                raise RuntimeError('Unexpected fixture archive member')
            target = output / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            with archive.extractfile(member) as source:
                target.write_bytes(source.read())


def measurements(path):
    data = path.read_bytes()
    result = {'relative_path': str(path), 'bytes': len(data),
              'file_sha256': hashlib.sha256(data).hexdigest()}
    if not (data.startswith(b'RIFF') and data[8:12] == b'WAVE'):
        if path.suffix == '.json':
            result['manifest'] = json.loads(data)
        return result
    probe = command([FFPROBE, '-v', 'error', '-show_format', '-show_streams',
                     '-of', 'json', str(path)], required=False)
    if probe.returncode:
        result['probe_error'] = probe.stderr.decode(errors='replace')
        return result
    result['probe'] = json.loads(probe.stdout)
    packets = command([FFPROBE, '-v', 'error', '-select_streams', 'a:0', '-show_packets',
                       '-show_data_hash', 'sha256', '-show_entries', 'packet=data_hash,size',
                       '-of', 'json', str(path)], required=False)
    result['packets'] = json.loads(packets.stdout) if packets.returncode == 0 else {
        'error': packets.stderr.decode(errors='replace')}
    covers = command([FFPROBE, '-v', 'error', '-select_streams', 'v', '-show_packets',
                      '-show_data_hash', 'sha256', '-show_entries', 'packet=stream_index,data_hash,size',
                      '-of', 'json', str(path)], required=False)
    result['cover_packets'] = json.loads(covers.stdout).get('packets', []) if covers.returncode == 0 else {
        'error': covers.stderr.decode(errors='replace')}
    pcm = command([FFMPEG, '-v', 'error', '-nostdin', '-i', str(path), '-map', '0:a:0',
                   '-c:a', 'pcm_s16le', '-f', 'hash', '-hash', 'sha256', '-'], required=False)
    result['decoded_pcm'] = pcm.stdout.decode().strip() if pcm.returncode == 0 else {
        'error': pcm.stderr.decode(errors='replace')}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('case', help='Unique receipt label; existing receipts are never replaced')
    parser.add_argument('--working', action='store_true', help='Also read generated OpenTag recovery records')
    args = parser.parse_args()
    if not re.fullmatch(r'[a-z0-9][a-z0-9-]{0,79}', args.case):
        parser.error('Use a short lowercase receipt label')
    actual = command([ADB, '-s', SERIAL, 'emu', 'avd', 'name']).stdout.decode().splitlines()[0]
    if actual != AVD:
        raise RuntimeError(f'Refusing device {actual}; expected disposable AVD {AVD}')
    output = ROOT / args.case
    output.mkdir(parents=True, exist_ok=False)
    collect('org.opentag.fixture.documents', 'files/documents', output / 'provider')
    if args.working:
        collect('org.opentag.editor', 'files/documents-v1', output / 'working')
    events = command([ADB, '-s', SERIAL, 'exec-out', 'run-as',
                      'org.opentag.fixture.documents', 'cat', 'files/events.jsonl'], required=False)
    receipt = {'created_utc': datetime.now(timezone.utc).isoformat(), 'serial': SERIAL, 'avd': actual,
               'scope': 'Read-only generated provider/working bytes; UI actions recorded separately',
               'events': events.stdout.decode(errors='replace'),
               'events_read_exit': events.returncode,
               'events_read_error': events.stderr.decode(errors='replace'),
               'files': [measurements(p) for p in sorted(output.rglob('*')) if p.is_file()]}
    for item in receipt['files']:
        item['relative_path'] = str(Path(item['relative_path']).relative_to(output))
    (output / 'receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + '\n')
    print(f'Recorded {len(receipt["files"])} generated files at {output / "receipt.json"}')


if __name__ == '__main__':
    main()
