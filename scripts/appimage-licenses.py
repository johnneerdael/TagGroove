#!/usr/bin/env python3
"""Stage shared license documents and inspect finished AppImages without running them."""
import argparse
import hashlib
import importlib.util
import io
import os
from pathlib import Path
import platform
import re
import subprocess
import tarfile
import urllib.request

_spec=importlib.util.spec_from_file_location('dependency_notices',Path(__file__).with_name('dependency-notices.py'))
core=importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(core)
require,sha,encoded=core.require,core.sha,core.encoded
SOURCES='src/licenses/appimage-common/SOURCES.json'
OUTPUT='target/appimage-common-licenses'
MANIFEST='target/appimage-license-stage.json'
OWNER='taggroove-appimage-licenses'


def name(value):
    require(isinstance(value,str) and re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9_.+-]*',value), 'Unsafe shared license name')
    return value


def source_spec(root):
    data=core.unique_json((root/SOURCES).read_bytes())
    require(data.get('schema')==1 and data.get('required_names')
            and len(data['required_names'])==len(set(data['required_names'])), 'Invalid shared-license source metadata')
    for value in data['required_names']:name(value)
    require({item['name'] for item in data['licenses']}=={'MPL-1.1','MPL-2.0'}
            and len(data['licenses'])==2, 'Unexpected Mozilla fallback set')
    inspector=data['inspector']
    require(re.fullmatch(r'[0-9]+(?:\.[0-9]+)+',inspector['version']), 'Unsafe inspector version')
    require({item['arch'] for item in inspector['archives']}=={'x86_64','aarch64'}
            and len(inspector['archives'])==2, 'Unexpected inspector architecture set')
    for item in inspector['archives']:
        name(item['archive'])
        require(item['url']=='https://github.com/ip7z/7zip/releases/download/'+inspector['version']+'/'+item['archive']
                and all(re.fullmatch('[a-f0-9]{64}',item.get(field,'')) for field in ['sha256','binary_sha256']),
                'Invalid inspector source/hash pin')
    return data


def safe_parents(root,path):
    for parent in path.parents:
        if parent==root:break
        require(not parent.is_symlink() and (not parent.exists() or parent.is_dir()), 'Unsafe staging parent')


def fallback_files(root,spec):
    files={}
    for item in spec['licenses']:
        raw=(root/core.safe(item['source_file'])).read_bytes()
        require(sha(raw)==item['sha256'], 'Mozilla fallback source hash differs')
        require(re.fullmatch('[a-f0-9]{40}',item['commit']) and '/'+item['commit']+'/' in item['source_url'],
                'Mozilla fallback is not pinned to an immutable source')
        require(hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()==item['git_blob_sha1'],
                'Mozilla fallback Git blob differs')
        raw.decode('utf-8')
        files[name(item['name'])]=(raw,'pinned fallback')
    return files


def validate_manifest(data):
    require(data.get('schema')==1 and data.get('owner')==OWNER and data.get('files'), 'Invalid shared-license stage manifest')
    for filename,record in data['files'].items():
        name(filename)
        require(re.fullmatch('[a-f0-9]{64}',record.get('sha256','')) and type(record.get('size')) is int
                and 0<record['size']<=1024*1024, 'Invalid shared-license file identity')


def stage(root,source):
    spec=source_spec(root)
    require(source.is_dir(), 'Missing host shared-license directory')
    source=source.resolve()
    files={}
    for path in sorted(source.iterdir()):
        filename=name(path.name)
        require(path.is_file() and path.resolve().is_relative_to(source), 'Host license alias escapes its source directory')
        raw=path.read_bytes()
        require(0<len(raw)<=1024*1024, 'Invalid host shared-license document size')
        raw.decode('utf-8')
        files[filename]=(raw,'host shared-license document')
    require(files, 'No host shared-license documents')
    for filename,value in fallback_files(root,spec).items():files.setdefault(filename,value)
    require(set(spec['required_names'])<=set(files), 'Host/fallback license collection is incomplete')
    destination=root/OUTPUT
    manifest_path=root/MANIFEST
    safe_parents(root,destination);safe_parents(root,manifest_path)
    require(not destination.is_symlink() and not manifest_path.is_symlink(), 'Unsafe shared-license stage path')
    guards={}
    if manifest_path.exists():
        original=manifest_path.read_bytes();old=core.unique_json(original);validate_manifest(old)
        guards[MANIFEST]=original
        actual={path.name for path in destination.iterdir()} if destination.is_dir() else set()
        require(actual<=set(old['files']), 'Unknown file in shared-license staging directory')
        for filename in actual:
            path=destination/filename
            require(path.is_file() and not path.is_symlink(), 'Nonregular managed shared-license file')
            raw=path.read_bytes();record=old['files'][filename]
            require(sha(raw)==record['sha256'] and len(raw)==record['size'], 'Managed shared-license file changed outside staging')
            guards[OUTPUT+'/'+filename]=raw
            # Retain previously managed standard terms when a host stops supplying
            # a name; no outside files are collected or removed.
            files.setdefault(filename,(raw,'previously staged standard document'))
    else:
        require(not destination.exists() or destination.is_dir() and not any(destination.iterdir()),
                'Unowned shared-license staging directory')
    data={'schema':1,'owner':OWNER,'files':{filename:{'sha256':sha(raw),'size':len(raw),'origin':origin}
          for filename,(raw,origin) in sorted(files.items())}}
    outputs={OUTPUT+'/'+filename:raw for filename,(raw,_origin) in sorted(files.items())}
    outputs[MANIFEST]=encoded(data)
    for relative in outputs:guards.setdefault(relative,None)
    core.publish_outputs(root,outputs,guards)
    require({path.name for path in destination.iterdir()}==set(files), 'Shared-license directory changed during staging')
    return data


def references(raw):
    text=raw.decode('utf-8')
    # Debian copyright prose often appends a full stop to the file reference.
    return {name(value.rstrip('.')) for value in re.findall(r'/usr/share/common-licenses/([A-Za-z0-9_.+-]+)',text)}


def verify_members(entries,manifest,read):
    validate_manifest(manifest)
    required={'usr/share/common-licenses/'+filename for filename in manifest['files']}
    actual={path for path in entries if path.startswith('usr/share/common-licenses/')
            and entries[path].get('Folder')=='-'}
    require(required==actual, 'Missing or unexpected packaged shared-license document')
    for filename,record in manifest['files'].items():
        path='usr/share/common-licenses/'+filename
        entry=entries[path]
        require(not entry.get('Symbolic Link') and not entry.get('Mode','').startswith('l'),
                'Packaged shared-license document must contain regular bytes')
        raw=read(path)
        require(sha(raw)==record['sha256'] and len(raw)==record['size'], 'Packaged shared-license hash differs: '+filename)
    copyrights=[path for path in entries if path.startswith('usr/share/doc/') and path.endswith('/copyright')]
    require(copyrights, 'AppImage has no bundled dependency copyright documents')
    referenced=set()
    for path in copyrights:
        require(entries[path].get('Folder')=='-' and not entries[path].get('Mode','').startswith('l'),
                'Unsupported copyright document kind')
        referenced.update(references(read(path)))
    require(referenced<=set(manifest['files']), 'Unresolved packaged shared-license reference: '+', '.join(sorted(referenced-set(manifest['files']))))
    return {'copyright_documents':len(copyrights),'referenced_licenses':sorted(referenced),'shared_documents':len(required)}


def inspector_record(root):
    require(platform.system()=='Linux', 'Prepared AppImage inspector is for Linux builders; pass --inspector elsewhere')
    arch={'arm64':'aarch64','amd64':'x86_64'}.get(platform.machine().lower(),platform.machine().lower())
    source=source_spec(root)['inspector']
    records=[item for item in source['archives'] if item['arch']==arch]
    require(len(records)==1, 'Unsupported AppImage inspector architecture')
    path=root/('target/appimage-inspector-'+source['version']+'-'+arch+'/7zz')
    safe_parents(root,path)
    return records[0],path


def prepare_inspector(root):
    record,path=inspector_record(root)
    if path.exists():
        require(path.is_file() and not path.is_symlink() and sha(path.read_bytes())==record['binary_sha256'],
                'Existing AppImage inspector bytes differ')
        os.chmod(path,0o755)
        return path
    with urllib.request.urlopen(record['url'],timeout=60) as response:raw=response.read()
    require(sha(raw)==record['sha256'], 'AppImage inspector archive checksum differs')
    with tarfile.open(fileobj=io.BytesIO(raw),mode='r:xz') as handle:
        member=handle.getmember('7zz');require(member.isfile(), 'Inspector archive has no regular executable')
        binary=handle.extractfile(member).read()
    require(sha(binary)==record['binary_sha256'], 'AppImage inspector executable checksum differs')
    relative=path.relative_to(root).as_posix()
    core.publish_outputs(root,{relative:binary},{relative:None})
    os.chmod(path,0o755)
    return path


def verify(root,package,inspector=None):
    manifest=core.unique_json((root/MANIFEST).read_bytes());validate_manifest(manifest)
    if inspector is None:
        record,inspector=inspector_record(root)
        require(inspector.is_file() and not inspector.is_symlink() and sha(inspector.read_bytes())==record['binary_sha256'],
                'Prepared AppImage inspector is missing or changed')
    command=[str(inspector),'l','-slt','-sccUTF-8',str(package)]
    result=subprocess.run(command,capture_output=True,check=False)
    require(result.returncode==0, 'Unable to inspect AppImage archive')
    entries={}
    for block in result.stdout.decode('utf-8').replace('\r\n','\n').split('\n\n'):
        entry=dict(line.split(' = ',1) for line in block.splitlines() if ' = ' in line)
        path=entry.get('Path','')
        if path.startswith('usr/'):
            core.safe(path);require(path not in entries, 'Duplicate AppImage path');entries[path]=entry
    def read(path):
        result=subprocess.run([str(inspector),'x','-so',str(package),path],capture_output=True,check=False)
        require(result.returncode==0, 'Unable to read packaged license material: '+path)
        return result.stdout
    return verify_members(entries,manifest,read)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[1])
    mode=parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--stage',action='store_true')
    mode.add_argument('--verify',type=Path)
    mode.add_argument('--verify-built',choices=['x86_64-unknown-linux-gnu','aarch64-unknown-linux-gnu'])
    parser.add_argument('--source-dir',type=Path,default=Path('/usr/share/common-licenses'))
    parser.add_argument('--prepare-inspector',action='store_true')
    parser.add_argument('--inspector',type=Path,help='Explicit existing inspection tool for local archive-only checks')
    args=parser.parse_args();root=args.root.resolve()
    try:
        if args.stage:
            data=stage(root,args.source_dir)
            if args.prepare_inspector:prepare_inspector(root)
            print('Staged '+str(len(data['files']))+' shared license documents')
        else:
            package=args.verify
            if args.verify_built:
                paths=list((root/'target'/args.verify_built/'release/bundle/appimage').glob('*.AppImage'))
                require(len(paths)==1, 'Expected one unambiguous built AppImage')
                package=paths[0]
            result=verify(root,package.resolve(),args.inspector.resolve() if args.inspector else None)
            print('Verified AppImage license material: '+str(result))
    except (OSError,ValueError,KeyError,TypeError,tarfile.TarError) as error:
        parser.exit(1,'AppImage licenses: '+str(error)+'\n')


if __name__=='__main__':main()
