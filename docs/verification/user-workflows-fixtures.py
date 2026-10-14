#!/usr/bin/env python3
"""Disposable OpenTag UAT media and independent file/audio receipts; no app IO."""
import argparse
import hashlib
import json
import re
import shutil
import struct
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path('/tmp/opentag-user-test')
AUDIO = {'.mp3', '.flac', '.m4a', '.opus', '.wav'}
FFMPEG = shutil.which('ffmpeg') or '/opt/homebrew/bin/ffmpeg'
FFPROBE = shutil.which('ffprobe') or '/opt/homebrew/bin/ffprobe'


def run(args):
    return subprocess.run(args, check=True, capture_output=True, text=True).stdout


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


def file_hash(path):
    digest = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(block)
    return digest.hexdigest()


def flac_comments(path):
    data = path.read_bytes()
    if data[:4] != b'fLaC':
        raise ValueError('Expected a FLAC stream')
    offset = 4
    while offset + 4 <= len(data):
        header = data[offset:offset + 4]
        size = int.from_bytes(header[1:], 'big')
        payload = data[offset + 4:offset + 4 + size]
        if header[0] & 127 == 4:
            vendor_length = struct.unpack_from('<I', payload)[0]
            cursor = 4 + vendor_length
            count = struct.unpack_from('<I', payload, cursor)[0]
            cursor += 4
            comments = []
            for _ in range(count):
                length = struct.unpack_from('<I', payload, cursor)[0]
                cursor += 4
                comments.append(payload[cursor:cursor + length].decode('utf-8'))
                cursor += length
            return comments
        offset += 4 + size
        if header[0] & 128:
            break
    return []


def set_flac_multivalues(path):
    """Author an ordinary Vorbis-comment block; retain the exact audio bytes."""
    data = path.read_bytes()
    offset = 4
    while offset + 4 <= len(data):
        header = data[offset:offset + 4]
        size = int.from_bytes(header[1:], 'big')
        payload = data[offset + 4:offset + 4 + size]
        if header[0] & 127 == 4:
            vendor_length = struct.unpack_from('<I', payload)[0]
            vendor = payload[4:4 + vendor_length]
            replaced = {'ARTIST', 'MOOD', 'TEST_A', 'TEST_B', 'UAT_REMOVE'}
            comments = [value for value in flac_comments(path) if value.split('=', 1)[0].upper() not in replaced]
            comments += ['ARTIST=Mémoire', 'ARTIST=Guest 猫', 'MOOD=Calm', 'MOOD=Focus',
                         'TEST_A=left', 'TEST_B=right', 'UAT_REMOVE=remove-me']
            encoded = [value.encode('utf-8') for value in comments]
            replacement = struct.pack('<I', len(vendor)) + vendor + struct.pack('<I', len(encoded))
            replacement += b''.join(struct.pack('<I', len(value)) + value for value in encoded)
            new_header = bytes([header[0]]) + len(replacement).to_bytes(3, 'big')
            path.write_bytes(data[:offset] + new_header + replacement + data[offset + 4 + size:])
            return
        offset += 4 + size
    raise ValueError('Generated FLAC has no Vorbis-comment block')


def receipt(library):
    files = []
    for path in sorted(library.rglob('*')):
        if not path.is_file() or path.suffix.lower() not in AUDIO:
            continue
        if path.is_symlink() or ROOT.resolve() not in path.resolve().parents:
            raise ValueError('Refusing media outside disposable UAT root')
        probe = json.loads(run([FFPROBE, '-v', 'error', '-show_format', '-show_streams', '-of', 'json', str(path)]))
        packets = json.loads(run([FFPROBE, '-v', 'error', '-select_streams', 'a:0', '-show_packets',
                                  '-show_data_hash', 'sha256', '-show_entries', 'packet=data_hash,size',
                                  '-of', 'json', str(path)]))['packets']
        packet_hashes = [packet['data_hash'] for packet in packets]
        covers = json.loads(run([FFPROBE, '-v', 'error', '-select_streams', 'v', '-show_packets',
                                 '-show_data_hash', 'sha256', '-show_entries', 'packet=stream_index,data_hash,size',
                                 '-of', 'json', str(path)])).get('packets', [])
        pcm = run([FFMPEG, '-v', 'error', '-i', str(path), '-map', '0:a:0', '-c:a', 'pcm_s16le', '-f', 'hash', '-hash', 'sha256', '-']).strip()
        files.append({
            'relative_path': str(path.relative_to(library)), 'size': path.stat().st_size,
            'file_sha256': file_hash(path), 'audio_packet_count': len(packets),
            'audio_packet_bytes': sum(int(packet['size']) for packet in packets),
            'audio_packet_sha256': hashlib.sha256(('\n'.join(packet_hashes) + '\n').encode()).hexdigest(),
            'packet_hashes': packet_hashes, 'decoded_s16le_pcm_sha256': pcm.removeprefix('SHA256='),
            'cover_packets': covers,
            'probe': probe, 'raw_flac_comments': flac_comments(path) if path.suffix.lower() == '.flac' else None,
        })
    return {'created_utc': datetime.now(timezone.utc).isoformat(), 'library': str(library), 'files': files}


def prepare():
    fixture = ROOT / 'fixtures'
    library = fixture / 'library'
    if library.exists():
        raise ValueError('Baseline already exists; use reset for a new case, never overwrite the baseline')
    for directory in [library / 'Disc 1', library / 'Disc 2', fixture / 'art', fixture / 'text', fixture / 'exports', fixture / 'actions', ROOT / 'work', ROOT / 'receipts']:
        directory.mkdir(parents=True, exist_ok=True)
    specifications = [
        ('Disc 1/01 - Solar Arcade - First Light.mp3', 220, 44100, 2, ['-c:a', 'libmp3lame', '-b:a', '192k'],
         {'title': 'First Light', 'artist': 'Solar Arcade', 'album': 'Morning Signals', 'album_artist': 'Solar Arcade', 'date': '2024', 'track': '01/06', 'disc': '1/2', 'genre': 'Electronic', 'comment': 'Original comment'}),
        ('Disc 1/02 - Solar Arcade - Night Shift.mp3', 330, 44100, 2, ['-c:a', 'libmp3lame', '-b:a', '192k'],
         {'title': 'Night Shift', 'artist': 'Solar Arcade', 'album': 'Morning Signals', 'album_artist': 'Solar Arcade', 'date': '2024', 'track': '02/06', 'disc': '1/2', 'genre': 'Electronic', 'comment': 'Second original'}),
        ('Disc 1/03 - Mémoire - Café 猫.flac', 440, 48000, 2, ['-c:a', 'flac'],
         {'title': 'Café 猫', 'artist': 'Mémoire', 'album': 'Archive Sessions', 'date': '2020-04-18', 'track': '03/06', 'disc': '1/2', 'genre': 'Ambient'}),
        ('Disc 2/04 - Solar Arcade - Quoted Title.m4a', 550, 44100, 2, ['-c:a', 'aac', '-b:a', '160k'],
         {'title': 'Title, with "quotes"', 'artist': 'Solar Arcade', 'album': 'Morning Signals', 'date': '2024', 'track': '4/6', 'disc': '2/2', 'genre': 'Pop'}),
        ('Disc 2/05 - Zephyr - Paper Planes.opus', 660, 48000, 1, ['-c:a', 'libopus', '-b:a', '96k'],
         {'title': 'Paper Paper Planes', 'artist': 'Zephyr', 'album': 'Open Sky', 'date': '2023', 'track': '5/6', 'disc': '2/2', 'genre': 'Indie'}),
        ('Disc 2/06 - Fixture Lab - Sanity Pulse.wav', 770, 44100, 1, ['-c:a', 'pcm_s16le'],
         {'title': 'Sanity Pulse', 'artist': 'Fixture Lab', 'album': 'Diagnostics', 'genre': 'Test'}),
    ]
    commands = []
    for relative, frequency, sample_rate, channels, codec, metadata in specifications:
        args = [FFMPEG, '-v', 'error', '-nostdin', '-f', 'lavfi', '-i', f'sine=frequency={frequency}:duration=3:sample_rate={sample_rate}', '-ac', str(channels), *codec]
        for key, value in metadata.items():
            args += ['-metadata', f'{key}={value}']
        args += [str(library / relative)]
        run(args)
        commands.append(args)
    set_flac_multivalues(library / specifications[2][0])
    for name, size, colour in [('front-600.png', '600x600', '0x264f74'), ('back-800x450.jpg', '800x450', '0xb77d39')]:
        args = [FFMPEG, '-v', 'error', '-nostdin', '-f', 'lavfi', '-i', f'color=c={colour}:s={size}',
                '-vf', 'drawbox=x=40:y=40:w=iw-80:h=ih-80:color=white:t=12', '-frames:v', '1', '-update', '1', str(fixture / 'art' / name)]
        run(args)
        commands.append(args)
    (fixture / 'text/lyrics-utf8.txt').write_text('UAT first line — Café 猫\nUAT second line\n', encoding='utf-8')
    (fixture / 'text/import-positional-utf8.txt').write_text('01|Solar Arcade|Imported First\n02|Solar Arcade|Imported Second\n', encoding='utf-8')
    (fixture / 'text/import-positional-utf16.txt').write_bytes(('\ufeff01|Solar Arcade|Imported First\r\n02|Solar Arcade|Imported Second\r\n').encode('utf-16-le'))
    (fixture / 'text/import-malformed.txt').write_text('01|Solar Arcade|Valid line\nmalformed\n', encoding='utf-8')
    (fixture / 'text/legacy-cp1252.txt').write_bytes('Café from a legacy text file\n'.encode('cp1252'))
    (fixture / 'exports/group-boundary.mte').write_text('$filename(txt,utf-8)$loop(%_filename_ext%)%_filename_ext%|%comment%\n$loopend()', encoding='utf-8')
    (fixture / 'exports/summary-csv.mte').write_text('$filename(csv,utf-8)"Filename";"Title"\n$loop(%_filename_ext%)"%_filename_ext%";"$replace(%title%,\'"\',\'""\')"\n$loopend()', encoding='utf-8')
    (fixture / 'actions/UAT-A-uppercase-comment.mta').write_text('[#0]\nT=5\nF=COMMENT\n1=$upper(%title%)\n', encoding='utf-8')
    (fixture / 'actions/UAT-B-prefix-comment.mta').write_text('[#0]\nT=5\nF=COMMENT\n1=B:%comment%\n', encoding='utf-8')
    (fixture / 'actions/UAT-unresolved.mta').write_text('[#0]\nT=999\nF=TITLE\n1=must-not-run\n', encoding='utf-8')
    write_json(ROOT / 'identifier-only.json', {'identifier': 'org.opentag.editor.usertest'})
    baseline = receipt(library)
    write_json(ROOT / 'receipts/baseline.json', baseline)
    inputs = []
    for path in sorted(fixture.rglob('*')):
        if path.is_file():
            item = {'path': str(path.relative_to(fixture)), 'size': path.stat().st_size, 'sha256': file_hash(path)}
            if path.suffix.lower() in {'.png', '.jpg'}:
                item['probe'] = json.loads(run([FFPROBE, '-v', 'error', '-show_entries', 'stream=codec_name,width,height', '-of', 'json', str(path)]))
            inputs.append(item)
    write_json(ROOT / 'receipts/fixture-inputs.json', {
        'status': 'fixtures prepared; native user workflows not executed',
        'generator': str(Path(__file__).resolve()), 'generator_sha256': file_hash(Path(__file__)),
        'ffmpeg': run([FFMPEG, '-version']).splitlines()[0],
        'ffprobe': run([FFPROBE, '-version']).splitlines()[0], 'audio_count': len(baseline['files']),
        'generation_commands': commands, 'inputs': inputs,
    })
    print(f'Prepared {len(baseline["files"])} actual audio files and inputs at {fixture}')


def checked_case(name):
    if not re.fullmatch(r'[a-z][a-z0-9-]{0,39}', name):
        raise ValueError('Case names must be short lowercase slugs')
    path = ROOT / 'work' / name
    if path.is_symlink() or path.resolve().parent != (ROOT / 'work').resolve():
        raise ValueError('Refusing a case path outside the disposable work directory')
    return path


def reset(name, force):
    case = checked_case(name)
    if case.exists():
        if not force:
            raise ValueError('Case exists; use a fresh name or explicitly --force after closing it in the app')
        shutil.rmtree(case)
    shutil.copytree(ROOT / 'fixtures/library', case / 'library')
    (case / 'outputs').mkdir()
    write_json(ROOT / 'receipts' / name / 'before.json', receipt(case / 'library'))
    print(f'Ready: {case / "library"}')


def capture(name, label):
    if not re.fullmatch(r'[a-z0-9][a-z0-9-]{0,49}', label):
        raise ValueError('Receipt label must be a short lowercase slug')
    case = checked_case(name)
    current = receipt(case / 'library')
    outputs = []
    for path in sorted((case / 'outputs').rglob('*')):
        if path.is_file():
            outputs.append({'path': str(path.relative_to(case)), 'size': path.stat().st_size, 'sha256': file_hash(path)})
    current['outputs'] = outputs
    destination = ROOT / 'receipts' / name / f'{label}.json'
    write_json(destination, current)
    before = json.loads((ROOT / 'receipts' / name / 'before.json').read_text(encoding='utf-8'))
    prior = {item['decoded_s16le_pcm_sha256']: item for item in before['files']}
    present = {item['decoded_s16le_pcm_sha256']: item for item in current['files']}
    missing = sorted(set(prior) - set(present))
    unexpected = sorted(set(present) - set(prior))
    changes = []
    packets_preserved = True
    for key in sorted(set(prior) & set(present)):
        old, new = prior[key], present[key]
        same_packets = old['audio_packet_sha256'] == new['audio_packet_sha256']
        packets_preserved &= same_packets
        changes.append({'before_path': old['relative_path'], 'after_path': new['relative_path'],
                        'file_hash_changed': old['file_sha256'] != new['file_sha256'],
                        'encoded_audio_packets_preserved': same_packets})
    summary = {'baseline_count': len(before['files']), 'current_count': len(current['files']),
               'missing_audio_pcm_hashes': missing, 'unexpected_audio_pcm_hashes': unexpected,
               'all_audio_payloads_preserved': not missing and not unexpected and packets_preserved and len(before['files']) == len(current['files']),
               'changes': changes, 'receipt': str(destination)}
    write_json(ROOT / 'receipts' / name / f'{label}-comparison.json', summary)
    print(json.dumps(summary, ensure_ascii=False, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('prepare')
    reset_parser = sub.add_parser('reset')
    reset_parser.add_argument('case')
    reset_parser.add_argument('--force', action='store_true')
    capture_parser = sub.add_parser('receipt')
    capture_parser.add_argument('case')
    capture_parser.add_argument('label')
    args = parser.parse_args()
    if args.command == 'prepare':
        prepare()
    elif args.command == 'reset':
        reset(args.case, args.force)
    else:
        capture(args.case, args.label)


if __name__ == '__main__':
    main()
