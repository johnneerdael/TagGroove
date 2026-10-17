"""Retain synthetic encoded fixtures and exact shipped-fpcalc oracle results."""
from pathlib import Path
import ast
import datetime
import hashlib
import importlib.util
import json
import os
import shutil
import subprocess
import sys
import wave

OUT = Path('/tmp/opentag-fp-goldens')
assert not OUT.exists(), 'Do not overwrite an existing evidence campaign'
OUT.mkdir()
fixtures = OUT / 'fixtures'
fixtures.mkdir()
logs = OUT / 'logs'
logs.mkdir()
sha = lambda path: hashlib.sha256(Path(path).read_bytes()).hexdigest()
reserve = lambda: shutil.disk_usage(OUT).free > 2.3 * 1024 ** 3 + 64 * 1024 ** 2
assert reserve()
oracle = Path('/opt/homebrew/Cellar/chromaprint/1.6.2/bin/fpcalc')
library = Path('/opt/homebrew/Cellar/chromaprint/1.6.2/lib/libchromaprint.1.dylib')
encoder = Path('/opt/homebrew/bin/ffmpeg').resolve()
assert sha(oracle) == '89d382fd61f67485a329fc73d1b76fc4d7b084c94d26fc2e19c8110a0da08d0b'
assert sha(library) == 'a9a451daec997fa88381ec15bd5c76df0a9432a62a8fbe2229dcbabccdd93db5'
original = Path('/tmp/opentag-fpcalc-native/probe_native.py')
bridge = Path('/tmp/opentag-fpcalc-native/probe_native_bridge.py')
source = original.read_text()
function = next(node for node in ast.parse(source).body if isinstance(node, ast.FunctionDef) and node.name == 'pcm')
recipe = 'import array\nimport math\nimport sys\n\n' + ast.get_source_segment(source, function) + '\n'
recipe_path = OUT / 'pcm_recipe.py'
recipe_path.write_text(recipe)
spec = importlib.util.spec_from_file_location('opentag_retained_pcm_recipe', recipe_path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
shutil.copyfile(__file__, OUT / 'generate.py')
env = {k: v for k, v in os.environ.items() if k not in ['GH_TOKEN', 'GITHUB_TOKEN', 'GH_ENTERPRISE_TOKEN', 'GITHUB_ENTERPRISE_TOKEN']}
report = {'schema': 1, 'at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'scope': 'Synthetic-only offline media generation and installed fpcalc reference execution. No OpenTag native/backend result asserted.', 'oracle': {'path': str(oracle), 'sha256': sha(oracle), 'library_path': str(library), 'library_sha256': sha(library)}, 'encoder': {'path': str(encoder), 'sha256': sha(encoder)}, 'generator': {'path': str(OUT / 'generate.py'), 'sha256': sha(OUT / 'generate.py'), 'pcm_recipe': {'path': str(recipe_path), 'sha256': sha(recipe_path), 'original_path': str(original), 'original_sha256': sha(original), 'bridge_recipe_path': str(bridge), 'bridge_recipe_sha256': sha(bridge)}}, 'cases': [], 'commands': []}

def run(label, command):
    assert reserve()
    result = subprocess.run([str(x) for x in command], env=env, capture_output=True, timeout=45)
    stdout, stderr = logs / (label + '.stdout'), logs / (label + '.stderr')
    stdout.write_bytes(result.stdout)
    stderr.write_bytes(result.stderr)
    record = {'label': label, 'argv': [str(x) for x in command], 'exit_code': result.returncode, 'stdout': str(stdout), 'stdout_sha256': sha(stdout), 'stderr': str(stderr), 'stderr_sha256': sha(stderr)}
    report['commands'].append(record)
    if result.returncode:
        raise RuntimeError(f'{label} failed: {result.returncode}: {result.stderr.decode(errors="replace")}')
    return result

report['encoder']['version'] = run('encoder-version', [encoder, '-version']).stdout.decode()
report['oracle']['version'] = run('oracle-version', [oracle, '-version']).stdout.decode()

def compare(name, path, details):
    before = sha(path)
    value = run(name + '-oracle', [oracle, '-json', path])
    expected = json.loads(value.stdout)
    assert expected['fingerprint'] and expected['duration'] > 0
    assert sha(path) == before
    record = {'name': name, 'file': str(path.relative_to(OUT)), 'bytes': path.stat().st_size, 'sha256': before, 'expected': expected, 'input_unchanged': True, **details}
    report['cases'].append(record)
    (OUT / 'reference-results.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'fixture': name, 'bytes': record['bytes'], 'duration': expected['duration']}), flush=True)

def make_wav(name, rate, channels, seconds, filename=None):
    assert reserve()
    samples = module.pcm(rate, channels, seconds)
    path = fixtures / (filename or (name + '.wav'))
    with wave.open(str(path), 'wb') as stream:
        stream.setnchannels(channels)
        stream.setsampwidth(2)
        stream.setframerate(rate)
        stream.writeframes(samples.tobytes())
    details = {'source': 'deterministic chirps/chords with changing amplitude envelope; no third-party recording', 'sample_rate': rate, 'channels': channels, 'input_frames': len(samples) // channels, 'requested_seconds': seconds, 'encoding': 'PCM signed16 little endian WAV'}
    compare(name, path, details)
    return path

source_wav = make_wav('stereo-wav', 44100, 2, 12, 'Synthetic grün muziek 音 12s.wav')
source_sha = sha(source_wav)
for name, suffix, options in [
    ('flac', '.flac', ['-c:a', 'flac']),
    ('mp3', '.mp3', ['-c:a', 'libmp3lame', '-q:a', '3']),
    ('aac-mp4', '.m4a', ['-c:a', 'aac']),
    ('alac-mp4', '.m4a', ['-c:a', 'alac']),
    ('opus', '.opus', ['-c:a', 'libopus']),
    ('vorbis', '.ogg', ['-c:a', 'vorbis', '-strict', 'experimental']),
    ('wavpack', '.wv', ['-c:a', 'wavpack']),
    ('tta', '.tta', ['-c:a', 'tta']),
    ('wma-v2', '.wma', ['-c:a', 'wmav2']),
    ('mpeg-layer2', '.mp2', ['-c:a', 'mp2']),
]:
    path = fixtures / (name + suffix)
    run(name + '-encode', [encoder, '-hide_banner', '-loglevel', 'error', '-i', source_wav, *options, path])
    assert sha(source_wav) == source_sha
    compare(name, path, {'source_fixture': str(source_wav.relative_to(OUT)), 'source_sha256': source_sha, 'encoding_argv': options})
make_wav('six-channel-wav', 48000, 6, 16)
make_wav('full135_37-prefix120', 11025, 1, 135.37)
make_wav('rounding8_495', 48000, 1, 8.495)
report['total_fixture_bytes'] = sum(row['bytes'] for row in report['cases'])
report['all_inputs_still_exact'] = all(sha(OUT / row['file']) == row['sha256'] for row in report['cases'])
assert report['all_inputs_still_exact'] and len(report['cases']) == 14
assert sha(oracle) == report['oracle']['sha256'] and sha(encoder) == report['encoder']['sha256']
(OUT / 'reference-results.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'status': 'PASS_REFERENCE_ONLY', 'cases': len(report['cases']), 'fixture_bytes': report['total_fixture_bytes'], 'report_sha256': sha(OUT / 'reference-results.json')}), flush=True)
