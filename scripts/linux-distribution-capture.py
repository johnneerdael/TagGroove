#!/usr/bin/env python3
"""Capture Linux package/source evidence; never run an inspected image or payload.

Offline replay authenticates Ubuntu repository selections and retained bytes.
Member extraction, installed-file comparison and ELF transformation equality are
trusted builder observations, not an independent offline reproduction.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform as host
import posixpath
import re
import shutil
import stat
import struct
import subprocess
import tempfile
import urllib.parse
import urllib.request


def helper(name, filename):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(filename))
    module = importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return module


material = helper('linux_source_material', 'linux-source-material.py')
packages = helper('linux_package_files', 'linux-package-files.py')
core = helper('dependency_notices', 'dependency-notices.py')
require, relative, sha = material.require, material.relative, material.sha
RECIPES = core.unique_json(Path(__file__).with_name('notices').joinpath('linux-packager-recipes.json').read_bytes())
TOOLS = core.unique_json(Path(__file__).with_name('notices').joinpath('linux-packager-tools.json').read_bytes())
PLATFORMS = {'linux-x64': ('amd64', 'x86_64', 'x86_64-linux-gnu', 'x86_64-unknown-linux-gnu'),
             'linux-arm64': ('arm64', 'aarch64', 'aarch64-linux-gnu', 'aarch64-unknown-linux-gnu')}
CACHES = {
    'usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache': ('usr/lib/gdk-pixbuf-2.0/2.10.0/loaders/', 'gdk-pixbuf-query-loaders'),
    'usr/lib/gio/modules/giomodule.cache': ('usr/lib/gio/modules/', 'gio-querymodules'),
    'usr/lib/gtk-3.0/3.0.0/immodules.cache': ('usr/lib/gtk-3.0/3.0.0/immodules/', 'gtk-query-immodules-3.0'),
    'usr/share/glib-2.0/schemas/gschemas.compiled': ('usr/share/glib-2.0/schemas/', 'glib-compile-schemas')}
OBSERVATION = 'builder-observed-full-byte-equality'
CAPTURE_OBSERVATIONS = 'trusted-builder-extraction-and-full-byte-replay'
CACHE_OBSERVATION = 'builder-observed-cache-bytes; recipe-derived-inputs; cache regeneration not claimed'
MAX_OBJECT = 512 * 1024 * 1024
MAX_MANIFEST = 16 * 1024 * 1024
MAX_INSPECTOR = 32 * 1024 * 1024


def bounded_file(path,limit):
    """Reject oversized/nonregular inputs before reading; detect size races."""
    info=path.lstat()
    require(stat.S_ISREG(info.st_mode) and 0<=info.st_size<=limit,'Unsafe or oversized capture input: '+str(path))
    with path.open('rb') as source:raw=source.read(info.st_size+1)
    require(len(raw)==info.st_size,'Capture input size changed during bounded read: '+str(path))
    return raw


def reference(raw, objects):
    digest = sha(raw)
    require(digest not in objects or objects[digest] == raw, 'Conflicting captured object digest')
    objects[digest] = raw
    return {'sha256': digest, 'size': len(raw)}


def object_bytes(record, objects, required):
    digest, size = record['sha256'], record['size']
    require(isinstance(digest, str) and re.fullmatch('[0-9a-f]{64}', digest)
            and type(size) is int and 0 <= size <= MAX_OBJECT, 'Invalid captured object reference')
    require(digest in objects and isinstance(objects[digest], bytes), 'Missing captured object: '+digest)
    raw = objects[digest]
    require(len(raw) == size and sha(raw) == digest, 'Captured object bytes differ: '+digest)
    required.add(digest)
    return raw


def parse_listing(text):
    require(text.count('----------\n') == 1, 'Ambiguous AppImage inspector listing')
    header, body = text.split('----------\n')
    offsets = re.findall(r'^Offset = ([0-9]+)$', header, re.M)
    require(len(offsets) == 1 and 'Type = SquashFS' in header, 'AppImage is not an inspected SquashFS image')
    entries = {}
    for paragraph in body.strip().split('\n\n'):
        if not paragraph.strip():continue
        fields = {}
        for line in paragraph.splitlines():
            require(' = ' in line, 'Malformed AppImage inspector record')
            key, value = line.split(' = ', 1)
            require(key not in fields, 'Repeated AppImage inspector field')
            fields[key] = value
        name = relative(fields['Path']);mode = fields['Mode']
        require(name not in entries and len(entries) < 100000, 'Duplicate or excessive AppImage paths')
        require(mode and mode[0] in '-ld', 'Unsupported AppImage member type')
        kind = {'-':'file', 'l':'symlink', 'd':'directory'}[mode[0]]
        size = fields.get('Size', '')
        require((kind == 'directory' and size in ('', '0')) or size.isdecimal(), 'Invalid AppImage member size')
        size = int(size or '0')
        require(size <= MAX_OBJECT, 'AppImage member exceeds bounds')
        entries[name] = {'kind':kind, 'size':size, 'mode':mode}
    require(entries, 'Empty AppImage member listing')
    return int(offsets[0]), entries


def digest_section(raw):
    require(raw.startswith(b'\x7fELF') and len(raw) >= 64 and raw[5] == 1, 'Unsupported runtime ELF encoding')
    try:
        if raw[4] == 2:
            table = struct.unpack_from('<Q', raw, 40)[0]
            width, count, names = struct.unpack_from('<HHH', raw, 58)
            minimum, offset_field, size_field, number = 64, 24, 32, '<Q'
        elif raw[4] == 1:
            table = struct.unpack_from('<I', raw, 32)[0]
            width, count, names = struct.unpack_from('<HHH', raw, 46)
            minimum, offset_field, size_field, number = 40, 16, 20, '<I'
        else:raise ValueError('Unsupported runtime ELF class')
        require(width >= minimum and 0 < count < 65535 and names < count
                and table+width*count <= len(raw), 'Runtime ELF section table exceeds bounds')
        def section(index):
            start = table+width*index
            return (struct.unpack_from('<I',raw,start)[0],
                    struct.unpack_from(number,raw,start+offset_field)[0],
                    struct.unpack_from(number,raw,start+size_field)[0])
        _, start, size = section(names)
        require(start+size <= len(raw), 'Runtime ELF section strings exceed bounds')
        strings = raw[start:start+size];matches = []
        for index in range(count):
            name, offset, size = section(index)
            require(name < len(strings), 'Invalid runtime ELF section name')
            if strings[name:].split(b'\0',1)[0] == b'.digest_md5':matches.append((offset,size))
        require(len(matches) == 1 and matches[0][1] == 16
                and matches[0][0]+16 <= len(raw), 'Runtime requires one sixteen-byte .digest_md5 section')
        return matches[0][0]
    except struct.error as error:raise ValueError('Truncated runtime ELF metadata') from error


def runtime_mask(observed, supplier):
    require(len(observed) == len(supplier), 'Runtime prefix size differs from supplier')
    offset = digest_section(supplier)
    require(digest_section(observed) == offset
            and observed[:offset] == supplier[:offset] and observed[offset+16:] == supplier[offset+16:],
            'AppImage runtime differs outside .digest_md5')
    return {'section':'.digest_md5','offset':offset,'size':16}


def generated_files(config, recipes):
    require(set(recipes) == {row['id'] for row in RECIPES['sources']}, 'Incomplete builder recipe sources')
    for row in RECIPES['sources']:
        require(sha(recipes[row['id']]) == row['sha256'], 'Pinned builder recipe bytes differ')
    require(config['productName'] == 'TagGroove' and config['bundle']['category'] == 'Music'
            and not config['bundle'].get('fileAssociations') and not config.get('plugins', {}).get('deep-link'),
            'Unsupported desktop generation configuration')
    comment = config['bundle']['shortDescription']
    require(isinstance(comment,str) and '\n' not in comment and '\r' not in comment, 'Unsafe desktop comment')
    template = recipes['tauri-desktop-template'].decode('utf-8')
    template = re.sub(r'{{#if comment}}\n(.*?){{/if}}\n', lambda match: match[1] if comment else '', template, flags=re.S)
    template = re.sub(r'{{#if mime_type}}\n.*?{{/if}}\n', '', template, flags=re.S)
    for name,value in {'categories':'AudioVideo;Audio;Music;','comment':comment,'exec':'taggroove','icon':'taggroove','name':'TagGroove'}.items():
        template = template.replace('{{'+name+'}}', value)
    require('{{' not in template, 'Unsupported desktop template placeholder')
    gtk = recipes['tauri-gtk-plugin']
    opening = b'cat > "$HOOKFILE" <<\\EOF\n'
    require(gtk.count(opening) == 1, 'GTK hook source template differs')
    hook = gtk.split(opening,1)[1].split(b'\nEOF\n',1)[0]+b'\n'
    exports = [('GSETTINGS_SCHEMA_DIR','/usr/share/glib-2.0/schemas'),
               ('GI_TYPELIB_PATH','/usr/lib/girepository-1.0'),('GIO_MODULE_DIR','/usr/lib/gio/modules'),
               ('GTK_EXE_PREFIX','/usr'),('GTK_PATH','/usr/lib/gtk-3.0'),
               ('GTK_IM_MODULE_FILE','/usr/lib/gtk-3.0/3.0.0/immodules.cache'),
               ('GDK_PIXBUF_MODULE_FILE','/usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache')]
    hook += ''.join(f'export {name}="$APPDIR/{path}"\n' for name,path in exports).encode()
    wrapper = (b'#! /usr/bin/env bash\n\n# autogenerated by linuxdeploy\n\n'
               b'# make sure errors in sourced scripts will cause this script to stop\nset -e\n\n'
               b'this_dir="$(readlink -f "$(dirname "$0")")"\n\n'
               b'source "$this_dir"/apprun-hooks/"linuxdeploy-plugin-gtk.sh"\n\n'
               b'exec "$this_dir"/AppRun.wrapped "$@"\n')
    return {'AppRun':wrapper,'apprun-hooks/linuxdeploy-plugin-gtk.sh':hook,
            'usr/share/applications/TagGroove.desktop':template.encode()}


def repository_replay(bindings, materials, objects, required, exact=True):
    """Re-authenticate stored repository inputs; fetch never reaches the network."""
    keyring = object_bytes(materials['repositories/ubuntu-keyring.gpg'], objects, required)
    snapshots, expected = {}, {}
    def fetcher(architecture,suite):
        origin = 'https://archive.ubuntu.com/ubuntu/' if architecture == 'amd64' else 'https://ports.ubuntu.com/ubuntu-ports/'
        def fetch(url,limit):
            require(url.startswith(origin), 'Stored repository fetch changed origin')
            path = relative(url[len(origin):])
            if '/by-hash/SHA256/' in path:
                digest=path.rsplit('/',1)[1]
                require(re.fullmatch('[0-9a-f]{64}',digest) and digest in objects, 'Missing stored repository by-hash object')
                raw=objects[digest]
                require(sha(raw)==digest,'Stored repository index digest differs')
                required.add(digest)
            elif path.startswith('dists/'):
                logical=f'repositories/{architecture}/{suite}/InRelease'
                require(path == f'dists/{suite}/InRelease' and logical in materials, 'Unknown stored repository request')
                raw=object_bytes(materials[logical],objects,required)
            else:
                logical='distribution/'+path
                require(logical in materials,'Missing stored source package material')
                raw=object_bytes(materials[logical],objects,required)
            require(len(raw)<=limit,'Stored repository input exceeds bounds')
            return raw
        return fetch
    seen=set()
    for binding in bindings:
        identity=(binding['suite'],binding['architecture'],binding['component'],binding['binary']['package'],binding['binary']['version'])
        require(identity not in seen,'Duplicate distribution package binding');seen.add(identity)
        key=identity[:2]
        if key not in snapshots:
            snapshots[key]=material.RepositorySnapshot(binding['architecture'],binding['suite'],keyring,fetcher(*key[::-1]))
        snapshot=snapshots[key]
        replay=snapshot.collect(binding['binary']['package'],binding['binary']['version'],binding['component'])
        require(replay == binding,'Distribution binding differs from authenticated source selection')
        object_bytes(binding['binary'],objects,required)
    for snapshot in snapshots.values():
        for name,raw in snapshot.materials.items():
            record={'sha256':sha(raw),'size':len(raw)}
            require(name not in expected or expected[name] == record,'Conflicting repository material path')
            expected[name]=record
    if exact:require(expected == materials,'Unexpected or unbound repository/source material')
    return expected


def verify_control(raw,binding):
    records=material.paragraphs(raw)
    require(len(records)==1,'Invalid retained binary control observation')
    record=records[0];binary=binding['binary']
    require(all(record.get(key)==binary[key] for key in ['package','version','architecture'])
            and material.binary_source(record)==(binary['source_package'],binary['source_version']),
            'Binary control observation differs from authenticated selection')


def verify_replay(record, raw, objects, required, path):
    original=object_bytes(record['original'],objects,required)
    replay=record['replay'];steps=replay['steps']
    require(record.get('observation')==OBSERVATION and replay['sha256']==sha(raw) and replay['size']==len(raw),
            'Missing full-byte builder replay observation')
    if record['kind']=='application-build':
        require(path=='usr/bin/taggroove' and steps in [['tauri-appimage-marker'],
                ['tauri-appimage-marker','set-rpath']], 'Unknown application packaging transformation')
        original=packages.appimage_application_input(original)
        steps=steps[1:]
    require(steps in [[],['strip'],['set-rpath'],['strip','set-rpath'],['webkit-relative-usr'],
                      ['strip','webkit-relative-usr'],['set-rpath','webkit-relative-usr'],
                      ['strip','set-rpath','webkit-relative-usr']], 'Unknown packaging transformation observation')
    if not steps:require(original==raw,'Full-copy observation bytes differ')
    else:
        require(original.startswith(b'\x7fELF') and raw.startswith(b'\x7fELF'),'Changed non-ELF observation')
        require(replay.get('expected_rpath')==expected_rpath(path,original),'ELF observation RPATH differs from builder recipe')
        require('webkit-relative-usr' not in steps or Path(path).name.startswith('libwebkit'),
                'WebKit transformation attached to another file family')
        require('strip' not in steps or posixpath.dirname(path)=='usr/lib',
                'Stripping attached to a preexisting AppImage member')


def cache_inputs(path,files):
    prefix,_=CACHES[path]
    return {name:record['sha256'] for name,record in files.items() if name.startswith(prefix)
            and record['kind']=='distribution-file' and (name.endswith(('.xml','.override','.dtd'))
            if path.endswith('gschemas.compiled') else name.endswith('.so'))}


def repository_asset(path,source):
    relative(source)
    return ((path=='TagGroove.png' or path.startswith('usr/share/icons/hicolor/'))
            and source.startswith('src-tauri/icons/') and source.endswith(('.png','.svg'))
            or path in {'usr/share/common-licenses/MPL-1.1','usr/share/common-licenses/MPL-2.0'}
            and source=='src/licenses/appimage-common/'+path.rsplit('/',1)[1])


def known_alias(path,target,files):
    require(not target.startswith('/'),'Absolute AppImage alias is not portable')
    resolved=packages.member_path(target,base=path.split('/')[:-1],link=True)
    fixed={'.DirIcon':'TagGroove.png','TagGroove.desktop':'usr/share/applications/TagGroove.desktop',
           'taggroove.svg':'usr/share/icons/hicolor/scalable/apps/taggroove.svg','usr/lib64':'usr/lib'}
    if path in fixed:require(resolved==fixed[path] and resolved in files,'Builder alias target differs')
    else:
        prefixes=('usr/lib/gtk-3.0/3.0.0/immodules/','usr/lib/gtk-3.0/3.0.0/printbackends/',
                  'usr/lib/gdk-pixbuf-2.0/2.10.0/loaders/')
        require(posixpath.dirname(path)=='usr/lib' and path.endswith('.so')
                and resolved.startswith(prefixes) and Path(path).name==Path(resolved).name
                and resolved in files and files[resolved]['kind']=='distribution-file',
                'Unknown or unbound AppImage alias')
    return resolved


def verify_materials(manifest, objects, runtime_spec, expected_commit, expected_version, expected_platform):
    require(manifest.get('schema')==1 and expected_platform in PLATFORMS
            and manifest.get('source_commit')==expected_commit and manifest.get('version')==expected_version
            and manifest.get('platform')==expected_platform, 'Distribution source/platform/release binding differs')
    architecture, runtime_arch, _, target=PLATFORMS[expected_platform]
    require(re.fullmatch('[0-9a-f]{40}',expected_commit) and manifest.get('target')==target
            and re.fullmatch('[0-9a-f]{64}',manifest['appimage_sha256'])
            and type(manifest['appimage_size']) is int and 0<manifest['appimage_size']<=1024**3
            and manifest.get('observations')==CAPTURE_OBSERVATIONS,'Invalid AppImage builder observation')
    require(manifest['tool_pins']==TOOLS['architectures'][architecture]['tools'],'Packaging tool pins differ')
    required=set();bindings=manifest['bindings']
    require(bindings and all(binding['architecture']==architecture for binding in bindings),'Wrong distribution binding architecture')
    repository_replay(bindings,manifest['materials'],objects,required)
    require(len(manifest['controls'])==len(bindings),'Missing package control observations')
    for binding,control in zip(bindings,manifest['controls']):verify_control(object_bytes(control,objects,required),binding)
    recipes={name:object_bytes(record,objects,required) for name,record in manifest['builder_recipes'].items()}
    inputs={relative(name):object_bytes(record,objects,required) for name,record in manifest['builder_inputs'].items()}
    require(set(inputs)=={'src-tauri/tauri.conf.json'},'Unknown builder configuration input')
    generated=generated_files(core.unique_json(inputs['src-tauri/tauri.conf.json']),recipes)
    supplier=object_bytes(manifest['runtime']['supplier'],objects,required)
    prefix=object_bytes(manifest['runtime']['prefix'],objects,required)
    launcher=object_bytes(manifest['launcher'],objects,required)
    pins=runtime_spec['asset_pins'][runtime_arch]
    require(sha(supplier)==pins['runtime']['sha256'] and len(supplier)==pins['runtime']['byte_length']
            and sha(launcher)==pins['launcher']['sha256'] and len(launcher)==pins['launcher']['byte_length'],
            'Runtime or launcher differs from pinned supplier asset')
    require(runtime_mask(prefix,supplier)==manifest['runtime']['mask'],'Runtime mask observation differs')
    files=manifest['files'];used_bindings=set();aliases={}
    require({'AppRun.wrapped','usr/bin/taggroove',*generated}<=set(files),'Missing required AppImage members')
    for path,record in files.items():
        relative(path);kind=record['kind']
        if kind=='directory':
            require(record.get('size')==0 and set(record)<={'kind','size','mode'},'Directory observation contains unbound bytes')
            continue
        raw=object_bytes(record,objects,required)
        if kind=='distribution-file':
            index=record['binding'];require(type(index) is int and 0<=index<len(bindings),'Unbound distribution member')
            used_bindings.add(index);relative(record['requested_member']);relative(record['resolved_member'])
            require(record['installed_path'].startswith('/') and relative(record['installed_path'][1:]),'Invalid installed source path')
            verify_replay(record,raw,objects,required,path)
        elif kind=='application-build':
            require(path=='usr/bin/taggroove' and record['source_commit']==expected_commit,'Unknown application build member')
            verify_replay(record,raw,objects,required,path)
        elif kind=='repository-bytes':
            require(repository_asset(path,record['source_file']) and record['source_commit']==expected_commit
                    and record.get('observation')=='builder-observed-repository-byte-equality', 'Unknown repository asset observation')
            require(hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()==record['git_blob_sha1'],
                    'Repository asset content identity differs')
        elif kind=='generated-builder':
            require(path in generated and raw==generated[path],'Generated builder bytes differ from pinned template')
        elif kind=='runtime-launcher':require(path=='AppRun.wrapped' and raw==launcher,'Unexpected launcher observation')
        elif kind=='symlink':
            aliases[path]=known_alias(path,raw.decode('utf-8'),files)
        elif kind=='generated-data':
            require(path in CACHES and record.get('observation')==CACHE_OBSERVATION,
                    'Unknown generated-data family')
            expected=cache_inputs(path,files)
            require(expected and record['inputs']==expected,'Generated cache input set differs')
            generator=record['generator'];index=generator['binding']
            require(type(index) is int and 0<=index<len(bindings),'Unbound generated cache tool')
            used_bindings.add(index);relative(generator['requested_member']);relative(generator['resolved_member'])
            require(Path(generator['requested_member']).name==CACHES[path][1],'Wrong generated cache tool')
            object_bytes(generator['original'],objects,required)
        else:raise ValueError('Unknown or unbound AppImage file kind: '+kind)
    require(used_bindings==set(range(len(bindings))),'Unreferenced distribution package binding')
    for path in aliases:
        seen=set();current=path
        while current in aliases:
            require(current not in seen,'Cyclic AppImage alias');seen.add(current);current=aliases[current]
    return required


def original_rpath(raw):
    """Read the authenticated input ELF's dynamic RPATH without executing it."""
    require(raw.startswith(b'\x7fELF') and len(raw)>=64 and raw[5]==1,'Unsupported original ELF encoding')
    try:
        if raw[4]==2:
            offset=struct.unpack_from('<Q',raw,32)[0];width,count=struct.unpack_from('<HH',raw,54)
            fmt,minimum,word,tag_format='<IIQQQQQQ',56,16,'<QQ'
        elif raw[4]==1:
            offset=struct.unpack_from('<I',raw,28)[0];width,count=struct.unpack_from('<HH',raw,42)
            fmt,minimum,word,tag_format='<IIIIIIII',32,8,'<II'
        else:raise ValueError('Unsupported original ELF class')
        require(width>=minimum and offset+width*count<=len(raw),'Original ELF program table exceeds bounds')
        loads=[];dynamic=None
        for index in range(count):
            fields=struct.unpack_from(fmt,raw,offset+width*index)
            if raw[4]==2:kind,file_offset,address,file_size=fields[0],fields[2],fields[3],fields[5]
            else:kind,file_offset,address,file_size=fields[0],fields[1],fields[2],fields[4]
            require(file_offset+file_size<=len(raw),'Original ELF segment exceeds bounds')
            if kind==1:loads.append((file_offset,address,file_size))
            if kind==2:
                require(dynamic is None,'Repeated original ELF dynamic segment');dynamic=(file_offset,file_size)
        if dynamic is None:return ''
        values={}
        for index in range(dynamic[0],dynamic[0]+dynamic[1],word):
            tag,value=struct.unpack_from(tag_format,raw,index)
            if tag==0:break
            if tag in {5,15,29}:require(tag not in values,'Repeated original ELF RPATH metadata');values[tag]=value
        selected=values.get(29,values.get(15))
        if selected is None:return ''
        require(5 in values,'Missing original ELF dynamic string table')
        locations=[start+values[5]-address for start,address,size in loads if address<=values[5]<address+size]
        require(len(locations)==1,'Unbound original ELF string table')
        start=locations[0]+selected;require(0<=start<len(raw),'Original ELF RPATH exceeds bounds')
        end=raw.find(b'\0',start,min(len(raw),start+65536));require(end>=start,'Unterminated original ELF RPATH')
        return raw[start:end].decode('utf-8')
    except struct.error as error:raise ValueError('Truncated original ELF metadata') from error


def expected_rpath(path,original=None):
    """Match the actual existing-file and newly deployed dependency branches.

    Pinned appdir.cpp833-861 treats every ELF recursively under usr/lib as an
    existing shared library, including WebKit helper executables. It preserves
    the original input RPATH and adds $ORIGIN; it does not use the explicit
    deployLibrary/deployExecutable destination branches for these copies.
    """
    relative(path)
    if path=='usr/bin/taggroove':return '$ORIGIN/../lib'
    if posixpath.dirname(path)=='usr/lib':return '$ORIGIN'
    previous=original_rpath(original) if original is not None and original.startswith(b'\x7fELF') else ''
    parts=previous.split(':') if previous else []
    if '$ORIGIN' not in parts:parts.append('$ORIGIN')
    expected=':'.join(parts)
    if Path(path).name.startswith('libwebkit'):expected=expected.replace('/usr','././')
    return expected


def host_command(arguments,limit):
    with tempfile.TemporaryFile() as output,tempfile.TemporaryFile() as errors:
        result=subprocess.run(list(map(str,arguments)),stdout=output,stderr=errors,timeout=120,
                              env={**os.environ,'LC_ALL':'C.UTF-8'})
        output.seek(0);errors.seek(0);raw=output.read(limit+1);message=errors.read(65537)
    require(result.returncode==0 and len(raw)<=limit and len(message)<=65536,'Host inspection command failed or exceeded bounds')
    return raw


def inspect_image(image,inspector,architecture):
    common=core.unique_json(Path(__file__).resolve().parents[1].joinpath('src/licenses/appimage-common/SOURCES.json').read_bytes())
    machine='x86_64' if architecture=='amd64' else 'aarch64'
    pins=[row for row in common['inspector']['archives'] if row['arch']==machine]
    require(len(pins)==1,'Missing pinned AppImage inspector architecture')
    inspector_limit=pins[0].get('binary_size',MAX_INSPECTOR)
    require(type(inspector_limit) is int and 0<inspector_limit<=MAX_INSPECTOR,'Invalid pinned inspector size bound')
    require(sha(bounded_file(inspector,inspector_limit))==pins[0]['binary_sha256'],
            'AppImage inspector differs from pinned host tool')
    listing=host_command([inspector,'l','-slt','--',image],16*1024*1024).decode('utf-8')
    offset,entries=parse_listing(listing)
    require(0<offset<=2*1024*1024,'AppImage runtime prefix exceeds bounds')
    with image.open('rb') as source:prefix=source.read(offset)
    require(len(prefix)==offset,'Truncated AppImage runtime prefix')
    contents={}
    for path,entry in entries.items():
        if entry['kind']=='directory':continue
        raw=host_command([inspector,'x','-so','--',image,path],entry['size'])
        require(len(raw)==entry['size'],'Inspected AppImage member size differs')
        contents[path]=raw
    return prefix,entries,contents


def installed_inventory(architecture):
    """Index installed package-owned files once, without guessing ownership."""
    fields='${binary:Package}\t${Version}\t${Architecture}\t${source:Package}\t${source:Version}\t${db:Status-Abbrev}\n'
    output=host_command(['dpkg-query','-W','-f='+fields],8*1024*1024).decode('utf-8')
    owners={}
    for line in output.splitlines():
        values=line.split('\t')
        require(len(values)==6,'Malformed installed package metadata')
        owner,version,arch,source,source_version,status=values
        if not status.startswith('ii') or arch not in {architecture,'all'}:continue
        owners[owner]={'package':owner.split(':')[0],'version':version,'architecture':arch,
                       'source_package':source or owner.split(':')[0],'source_version':source_version or version}
    index={}
    for listing in sorted(Path('/var/lib/dpkg/info').glob('*.list')):
        owner=listing.name[:-5]
        metadata=owners.get(owner)
        if metadata is None:continue
        for name in listing.read_text(encoding='utf-8').splitlines():
            if not name.startswith(('/usr/lib/','/lib/','/usr/libexec/','/usr/share/doc/',
                                    '/usr/share/common-licenses/','/usr/share/glib-2.0/schemas/','/usr/bin/')):continue
            path=Path(name)
            if path.is_file():
                canonical=str(path.resolve())
                index.setdefault(name,[]).append({**metadata,'path':name,'canonical':canonical})
    require(index,'No installed distribution file inventory')
    return index


def read_installed(candidate):
    path=Path(candidate['canonical'])
    require(path.is_file() and path.stat().st_size<=MAX_OBJECT,'Installed source file exceeds bounds')
    return path.read_bytes()


def candidates(index,path,triplet,basename=False):
    names={'/'+path}
    if path.startswith('usr/lib/'):
        suffix=path[len('usr/lib/'):]
        names.update({'/usr/lib/'+triplet+'/'+suffix,'/lib/'+triplet+'/'+suffix})
        basename=basename or '/' not in suffix
    result=[]
    for name,rows in index.items():
        if name in names or basename and Path(name).name==Path(path).name:result.extend(rows)
    # Copyright files can be leaf aliases into another package's documentation.
    # Resolve ownership of the canonical target, then prove that package member.
    for canonical in {row['canonical'] for row in result}:result.extend(index.get(canonical,[]))
    # A package may list only a documentation-directory symlink (libatomic1
    # points to gcc-14-base). Its copyright leaf then has no separate .list row.
    if path.startswith('usr/share/doc/') and path.endswith('/copyright'):
        try:canonical=str(Path('/'+path).resolve(strict=True))
        except FileNotFoundError:canonical=None
        if canonical is not None and canonical.startswith('/usr/share/doc/'):
            result.extend(index.get(canonical,[]))
    unique={(row['path'],row['package'],row['version']):row for row in result}
    return [unique[key] for key in sorted(unique)]


def repository_snapshots(architecture):
    keyring=Path('/usr/share/keyrings/ubuntu-archive-keyring.gpg').read_bytes()
    return [material.RepositorySnapshot(architecture,suite,keyring) for suite in ['noble-updates','noble-security','noble']]


def package_control(raw):
    tool=shutil.which('dpkg-deb');require(tool is not None,'Package control verification requires dpkg-deb')
    with tempfile.TemporaryDirectory(prefix='taggroove-capture-control-') as folder:
        path=Path(folder)/'input.deb';path.write_bytes(raw)
        _,bodies=packages.scan_payload(tool,path,{'control'},control=True)
    require(bodies.get('control'),'Missing captured package control')
    return bodies['control']


def repository_sources(root,path,raw,config):
    possible=[]
    if path in {'usr/share/common-licenses/MPL-1.1','usr/share/common-licenses/MPL-2.0'}:
        spec=core.unique_json((root/'src/licenses/appimage-common/SOURCES.json').read_bytes())
        for row in spec['licenses']:
            if row['name']==Path(path).name:
                source=root/relative(row['source_file']);content=source.read_bytes()
                require(sha(content)==row['sha256'],'Tracked common-license fallback differs from source pin')
                if content==raw:possible.append(row['source_file'])
    if path=='TagGroove.png' or path.startswith('usr/share/icons/hicolor/'):
        paths=[root/'src-tauri/icons/linux'/path.removeprefix('usr/share/icons/')]
        paths += [root/'src-tauri'/relative(name) for name in config['bundle']['icon'] if name.endswith('.png')]
        if path=='TagGroove.png':paths+=list((root/'src-tauri/icons/linux/hicolor').rglob('taggroove.png'))
        for source in paths:
            if source.is_file() and source.read_bytes()==raw:possible.append(source.relative_to(root).as_posix())
    return sorted(set(possible))


def pinned_runtime(root,architecture,pin):
    cached=root/'target/linux-source-tools'/architecture/'runtime'
    if cached.is_file():raw=bounded_file(cached,pin['byte_length'])
    else:
        url=pin['url'];require(urllib.parse.urlsplit(url).scheme=='https','Runtime download requires HTTPS')
        with urllib.request.urlopen(url,timeout=60) as response:
            require(urllib.parse.urlsplit(response.geturl()).scheme=='https','Runtime download downgraded HTTPS')
            raw=response.read(pin['byte_length']+1)
    require(len(raw)==pin['byte_length'] and sha(raw)==pin['sha256'],'Runtime supplier bytes differ from pin')
    return raw


def capture(root, platform, source_commit, version, image, inspector, tools, tool_pins):
    require(host.system()=='Linux','Distribution capture must run on a Linux builder')
    require(platform in PLATFORMS and re.fullmatch('[0-9a-f]{40}',source_commit),'Invalid Linux capture identity')
    architecture,runtime_arch,triplet,target=PLATFORMS[platform]
    require(tool_pins==TOOLS['architectures'][architecture]['tools'],'Capture packaging tool pins differ')
    require(image.is_file() and not image.is_symlink() and 0<image.stat().st_size<=1024**3,'Invalid AppImage capture input')
    image_raw=image.read_bytes();image_hash=sha(image_raw)
    prefix,entries,contents=inspect_image(image,inspector,architecture)
    require(set(contents)=={name for name,row in entries.items() if row['kind']!='directory'},'Incomplete AppImage inspection')
    for name,row in entries.items():
        relative(name)
        if row['kind']!='directory':require(len(contents[name])==row['size'],'AppImage inspection member size differs')
    runtime_spec=core.unique_json((root/'scripts/notices/linux-runtime-material.json').read_bytes())
    supplier=pinned_runtime(root,architecture,runtime_spec['asset_pins'][runtime_arch]['runtime'])
    mask=runtime_mask(prefix,supplier);objects={};files={};bindings=[];controls=[];contexts={}
    recipes={row['id']:reference((root/row['source_file']).read_bytes(),objects) for row in RECIPES['sources']}
    config_raw=(root/'src-tauri/tauri.conf.json').read_bytes();config=core.unique_json(config_raw)
    generated=generated_files(config,{name:objects[row['sha256']] for name,row in recipes.items()})
    index=installed_inventory(architecture);snapshots=repository_snapshots(architecture)

    def context(candidate):
        key=(candidate['package'],candidate['version'])
        if key not in contexts:
            found=None
            for snapshot in snapshots:
                for component in sorted(snapshot.components):
                    try:selection=snapshot._binary(*key,component)
                    except ValueError as error:
                        if str(error)=='Expected one exact repository binary package':continue
                        raise
                    require((selection['source_package'],selection['source_version'])==
                            (candidate['source_package'],candidate['source_version']),
                            'Installed source version differs from authenticated binary selection')
                    binding=snapshot.collect(*key,component);binary=snapshot.binary_bytes(binding)
                    control=package_control(binary);verify_control(control,binding)
                    found=(binding,binary,control);break
                if found is not None:break
            require(found is not None,'Installed binary/source version is unavailable in authenticated repositories: '+str(key))
            contexts[key]=found
        return contexts[key]

    def register(ctx):
        binding,binary,control=ctx
        reference(binary,objects)
        if binding not in bindings:
            bindings.append(binding);controls.append(reference(control,objects))
        return bindings.index(binding)

    def prove(rows,raw,path,generator=False):
        matches={}
        for candidate in rows:
            ctx=context(candidate);binding,binary,_=ctx
            requested=relative(candidate['path'].removeprefix('/'))
            try:member=packages.package_members(binary,binding['binary'],[requested])[requested]
            except ValueError:continue
            original=member['bytes']
            if read_installed(candidate)!=original:continue
            if generator:replay=None
            else:
                try:
                    replay=packages.replay_packaging(original,raw,tools,tool_pins,
                        webkit_relative_usr=Path(path).name.startswith('libwebkit'),expected_rpath=expected_rpath(path,original),
                        strip_input=posixpath.dirname(path)=='usr/lib')
                except ValueError:continue
                replay['expected_rpath']=expected_rpath(path,original)
            key=(json.dumps(binding,sort_keys=True),member['path'],sha(original))
            matches[key]=(candidate,ctx,requested,member,original,replay)
        require(len(matches)==1,'Unknown or ambiguous packaged/installed file: '+path)
        candidate,ctx,requested,member,original,replay=next(iter(matches.values()))
        result={'binding':register(ctx),'installed_path':candidate['canonical'],'requested_member':requested,
                'resolved_member':member['path'],'original':reference(original,objects)}
        if replay is not None:result.update(replay=replay,observation=OBSERVATION)
        return result

    for path,entry in entries.items():
        if entry['kind']=='directory':files[path]={'kind':'directory','size':0,'mode':entry['mode']};continue
        raw=contents[path];record=reference(raw,objects)
        if entry['kind']=='symlink':files[path]={'kind':'symlink',**record};continue
        if path=='AppRun.wrapped':files[path]={'kind':'runtime-launcher',**record}
        elif path=='usr/bin/taggroove':
            original_path=root/'target'/target/'release/taggroove'
            if not original_path.is_file():original_path=root/'target/release/taggroove'
            require(original_path.is_file(),'Missing own application build input')
            original=original_path.read_bytes()
            replay=packages.replay_application(original,raw,tools,tool_pins)
            replay['expected_rpath']=expected_rpath(path)
            files[path]={'kind':'application-build',**record,'original':reference(original,objects),
                         'source_commit':source_commit,'replay':replay,'observation':OBSERVATION}
        elif path in generated:
            require(raw==generated[path],'Generated AppImage member differs from pinned builder recipe: '+path)
            files[path]={'kind':'generated-builder',**record}
        elif path in CACHES:
            files[path]={'kind':'generated-data',**record}
        else:
            sources=repository_sources(root,path,raw,config)
            if sources:
                files[path]={'kind':'repository-bytes',**record,'source_file':sources[0],'source_commit':source_commit,
                             'git_blob_sha1':hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest(),
                             'observation':'builder-observed-repository-byte-equality'}
            else:files[path]={'kind':'distribution-file',**record,**prove(candidates(index,path,triplet),raw,path)}
    for path,record in files.items():
        if record['kind']!='generated-data':continue
        generator=CACHES[path][1]
        record.update(inputs=cache_inputs(path,files),
                      generator=prove(candidates(index,'usr/bin/'+generator,triplet,basename=True),b'',generator,True),
                      observation=CACHE_OBSERVATION)
    all_materials={}
    for snapshot in snapshots:
        for name,raw in snapshot.materials.items():
            require(name not in all_materials or all_materials[name]==raw,'Conflicting captured repository material')
            all_materials[name]=raw
    material_refs={name:reference(raw,objects) for name,raw in all_materials.items()}
    minimal=repository_replay(bindings,material_refs,objects,set(),exact=False)
    require('AppRun.wrapped' in contents,'Missing packaged launcher')
    manifest={'schema':1,'source_commit':source_commit,'version':version,'platform':platform,'target':target,
              'appimage_sha256':image_hash,'appimage_size':len(image_raw),'materials':minimal,'bindings':bindings,
              'controls':controls,'files':files,'builder_recipes':recipes,
              'builder_inputs':{'src-tauri/tauri.conf.json':reference(config_raw,objects)},'tool_pins':tool_pins,
              'runtime':{'supplier':reference(supplier,objects),'prefix':reference(prefix,objects),'mask':mask},
              'launcher':reference(contents['AppRun.wrapped'],objects),'observations':CAPTURE_OBSERVATIONS}
    required=verify_materials(manifest,objects,runtime_spec,source_commit,version,platform)
    require(sha(image.read_bytes())==image_hash,'AppImage changed during source capture')
    return manifest,{digest:objects[digest] for digest in required}


def export_capture(directory,manifest,objects):
    require(manifest['platform'] in PLATFORMS,'Invalid export platform')
    require(not directory.is_symlink() and not (directory/'objects').is_symlink(),'Unsafe capture export directory')
    def observed(path,limit):
        try:return bounded_file(path,limit)
        except FileNotFoundError:return None
    manifest_name=f"linux-source-{manifest['platform']}.json"
    guards={manifest_name:observed(directory/manifest_name,MAX_MANIFEST)}
    outputs={manifest_name:core.encoded(manifest)}
    for digest,raw in objects.items():
        require(re.fullmatch('[0-9a-f]{64}',digest) and sha(raw)==digest,'Invalid exported object digest')
        name='objects/'+digest;path=directory/name
        previous=observed(path,len(raw))
        if previous is not None:require(previous==raw,'Existing object digest collision')
        else:
            outputs[name]=raw
            guards[name]=None
    directory.mkdir(parents=True,exist_ok=True)
    core.publish_outputs(directory,outputs,guards)


def source_files(evidence_dir,commit,version,runtime_spec):
    require(evidence_dir.is_dir() and not evidence_dir.is_symlink()
            and (evidence_dir/'objects').is_dir() and not (evidence_dir/'objects').is_symlink(),
            'Missing or unsafe Linux capture evidence directory')
    objects={}
    for path in (evidence_dir/'objects').iterdir():
        require(re.fullmatch('[0-9a-f]{64}',path.name),'Unsafe Linux capture object filename')
        raw=bounded_file(path,MAX_OBJECT)
        require(sha(raw)==path.name,'Linux capture object file hash differs');objects[path.name]=raw
    result={};required=set()
    for platform in PLATFORMS:
        path=evidence_dir/f'linux-source-{platform}.json'
        require(path.is_file() and not path.is_symlink(),'Missing Linux target capture manifest: '+platform)
        raw=bounded_file(path,MAX_MANIFEST);manifest=core.unique_json(raw)
        required.update(verify_materials(manifest,objects,runtime_spec,commit,version,platform))
        result[f'linux-distribution/targets/{platform}.json']=raw
    require(required==set(objects),'Unexpected or unbound Linux capture objects')
    result.update({'linux-distribution/objects/'+digest:objects[digest] for digest in sorted(required)})
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[1])
    parser.add_argument('--platform',choices=PLATFORMS,required=True)
    parser.add_argument('--source-commit',required=True);parser.add_argument('--version',required=True)
    parser.add_argument('--image',type=Path,required=True);parser.add_argument('--inspector',type=Path,required=True)
    parser.add_argument('--strip',type=Path,required=True);parser.add_argument('--patchelf',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    try:
        pins=TOOLS['architectures'][PLATFORMS[args.platform][0]]['tools']
        manifest,objects=capture(args.root.resolve(),args.platform,args.source_commit,args.version,
                                 args.image.resolve(),args.inspector.resolve(),{'strip':args.strip,'patchelf':args.patchelf},pins)
        export_capture(args.output.resolve(),manifest,objects)
        print(f"Captured {args.platform}: {len(manifest['files'])} paths, {len(manifest['bindings'])} package/source bindings")
    except (OSError,ValueError,KeyError,TypeError,subprocess.SubprocessError) as error:
        parser.exit(1,f'Linux distribution capture: {error}\n')


if __name__=='__main__':main()
