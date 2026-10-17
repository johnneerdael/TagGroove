from pathlib import Path
import datetime
import gzip
import hashlib
import json
import shutil

root = Path('/tmp/opentag-fp-goldens')
source = root / 'reference-results.json'
digest = lambda path: hashlib.sha256(Path(path).read_bytes()).hexdigest()
assert digest(source) == 'bebe84030c001aff4ccb08dfedc037b48c6b99ea114e19f785a81dbd08a21def'
records = json.loads(source.read_text())
output = root / 'carriers'
output.mkdir()
cases = []
for row in records['cases']:
    original = root / row['file']
    assert digest(original) == row['sha256']
    compressed = original.suffix == '.wav'
    carrier = output / (original.name + ('.gz' if compressed else ''))
    if compressed:
        with original.open('rb') as reader, carrier.open('wb') as writer:
            with gzip.GzipFile(filename='', mode='wb', fileobj=writer, compresslevel=9, mtime=0) as zipped:
                shutil.copyfileobj(reader, zipped)
        assert carrier.read_bytes()[4:8] == b'\x00\x00\x00\x00'
        digest_stream = hashlib.sha256()
        count = 0
        with gzip.open(carrier, 'rb') as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b''):
                count += len(chunk)
                digest_stream.update(chunk)
        assert count == row['bytes'] and digest_stream.hexdigest() == row['sha256']
    else:
        shutil.copyfile(original, carrier)
        assert digest(carrier) == row['sha256']
    assert digest(original) == row['sha256']
    cases.append({**row, 'carrier': str(carrier.relative_to(root)), 'carrier_encoding': 'gzip' if compressed else 'identity', 'carrier_bytes': carrier.stat().st_size, 'carrier_sha256': digest(carrier), 'decoded_bytes': row['bytes'], 'decoded_sha256': row['sha256'], 'lossless_decode_verified': True})
result = {'schema': 1, 'at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'reference': {'path': str(source), 'sha256': digest(source)}, 'source_generator': records['generator'], 'oracle': records['oracle'], 'encoder': records['encoder'], 'carrier_generator_sha256': digest(__file__), 'cases': cases, 'original_bytes': sum(r['bytes'] for r in cases), 'carrier_bytes': sum(r['carrier_bytes'] for r in cases), 'all_originals_preserved': True, 'native_api_or_cross_platform_result_claimed': False}
(root / 'carrier-results.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'cases': len(cases), 'original_bytes': result['original_bytes'], 'carrier_bytes': result['carrier_bytes'], 'manifest': str(root / 'carrier-results.json'), 'sha256': digest(root / 'carrier-results.json')}))
