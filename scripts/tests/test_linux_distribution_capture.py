"""Distribution capture boundaries and offline provenance refusal tests."""
import copy
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import struct
import tempfile
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1]/'linux-distribution-capture.py'
ROOT = SCRIPT.parents[1]


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def runtime_elf():
    raw = bytearray(384)
    raw[:6] = b'\x7fELF\x02\x01'
    struct.pack_into('<Q', raw, 40, 128)
    struct.pack_into('<HHH', raw, 58, 64, 3, 1)
    names = b'\0.shstrtab\0.digest_md5\0'
    raw[320:320+len(names)] = names
    struct.pack_into('<IQQ', raw, 128+64, 1, 0, 0)
    struct.pack_into('<QQ', raw, 128+64+24, 320, len(names))
    struct.pack_into('<I', raw, 128+128, 11)
    struct.pack_into('<QQ', raw, 128+128+24, 352, 16)
    return bytes(raw)


def elf_with_rpath(value):
    raw=bytearray(512);raw[:6]=b'\x7fELF\x02\x01'
    struct.pack_into('<Q',raw,32,64);struct.pack_into('<HH',raw,54,56,2)
    struct.pack_into('<IIQQQQQQ',raw,64,1,4,0,0x400000,0x400000,512,512,4096)
    struct.pack_into('<IIQQQQQQ',raw,120,2,4,192,0x4000c0,0x4000c0,64,64,8)
    struct.pack_into('<QQQQQQ',raw,192,5,0x400100,29,1,0,0)
    text=b'\0'+value.encode()+b'\0';raw[256:256+len(text)]=text
    return bytes(raw)


class LinuxDistributionCapture(unittest.TestCase):
    def module(self):
        self.assertTrue(SCRIPT.is_file(), 'Missing Linux distribution capture helper')
        spec = importlib.util.spec_from_file_location('linux_distribution_capture', SCRIPT)
        module = importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        return module

    def test_listing_rejects_duplicate_unsafe_and_unsupported_entries(self):
        m = self.module()
        header = 'Type = SquashFS\nOffset = 384\n----------\n'
        row = 'Path = usr/lib/example.so\nFolder = -\nSize = 3\nMode = -rwxr-xr-x\n\n'
        offset, entries = m.parse_listing(header+row)
        self.assertEqual(offset, 384)
        self.assertEqual(entries['usr/lib/example.so'], {'kind':'file','size':3,'mode':'-rwxr-xr-x'})
        for text in [header+row+row, header+row.replace('usr/lib/example.so','../outside'),
                     header+row.replace('-rwxr-xr-x','prw-r--r--'),
                     header+row.replace('Size = 3','Size = -1')]:
            with self.assertRaises(ValueError):m.parse_listing(text)

    def test_runtime_mask_changes_only_the_exact_sixteen_byte_digest_section(self):
        m = self.module();supplier = runtime_elf();packaged = bytearray(supplier)
        packaged[352:368] = b'packager digest!'
        # Supply exactly sixteen bytes; no other header/data difference is permitted.
        packaged[352:368] = b'0123456789abcdef'
        self.assertEqual(m.runtime_mask(bytes(packaged),supplier), {'section':'.digest_md5','offset':352,'size':16})
        for position in [0,100,351,368]:
            changed = bytearray(packaged);changed[position] ^= 1
            with self.subTest(position=position), self.assertRaises(ValueError):m.runtime_mask(bytes(changed),supplier)
        with self.assertRaises(ValueError):m.runtime_mask(bytes(packaged)+b'x',supplier)

    def test_generated_templates_are_exact_and_gtk_hook_is_bound_to_pinned_plugin_source(self):
        m = self.module();recipes = {r['id']:(ROOT/r['source_file']).read_bytes() for r in m.RECIPES['sources']}
        config = json.loads((ROOT/'src-tauri/tauri.conf.json').read_bytes())
        generated = m.generated_files(config,recipes)
        self.assertEqual(set(generated), {'AppRun','apprun-hooks/linuxdeploy-plugin-gtk.sh',
                                         'usr/share/applications/TagGroove.desktop'})
        self.assertEqual(generated['usr/share/applications/TagGroove.desktop'],
                         b'[Desktop Entry]\nCategories=AudioVideo;Audio;Music;\nComment=Open source audio metadata editor\nExec=taggroove\nStartupWMClass=taggroove\nIcon=taggroove\nName=TagGroove\nTerminal=false\nType=Application\n')
        self.assertIn(b'exec "$this_dir"/AppRun.wrapped "$@"\n',generated['AppRun'])
        self.assertIn(b'export GI_TYPELIB_PATH="$APPDIR//usr/lib/girepository-1.0"\n',
                      generated['apprun-hooks/linuxdeploy-plugin-gtk.sh'])
        changed=dict(recipes);changed['tauri-gtk-plugin'] += b'changed'
        with self.assertRaises(ValueError):m.generated_files(config,changed)

    @unittest.skipUnless(shutil.which('gpgv'), 'GnuPG signature verifier required')
    def repository(self,m,architecture='amd64'):
        folder = ROOT/'scripts/tests/fixtures/linux-source'
        files = {name:(folder/name).read_bytes() for name in [
            'repository-InRelease.asc','repository-Sources.xz','repository-Sources-universe.xz',
            'repository-Packages-amd64.xz','repository-Packages-arm64.xz','repository-keyring.gpg',
            'example_1.2.orig.tar.xz','example_1.2-3.debian.tar.xz','example_1.2-3.dsc',
            'libexample1_1.2-3+b1_amd64.deb.fixture']}
        paths = {'dists/noble-updates/InRelease':files['repository-InRelease.asc'],
                 'dists/noble-updates/main/source/Sources.xz':files['repository-Sources.xz'],
                 'dists/noble-updates/universe/source/Sources.xz':files['repository-Sources-universe.xz'],
                 f'dists/noble-updates/main/binary-{architecture}/Packages.xz':files[f'repository-Packages-{architecture}.xz']}
        for name in ['example_1.2.orig.tar.xz','example_1.2-3.debian.tar.xz','example_1.2-3.dsc']:
            paths['pool/main/e/example/'+name] = files[name]
        paths['pool/main/e/example/libexample1_1.2-3+b1_amd64.deb'] = files['libexample1_1.2-3+b1_amd64.deb.fixture']
        by_hash={sha(raw):raw for raw in paths.values()}
        def fetch(url,limit):
            origin='https://archive.ubuntu.com/ubuntu/' if architecture=='amd64' else 'https://ports.ubuntu.com/ubuntu-ports/'
            path=url.removeprefix(origin)
            raw=by_hash[path.rsplit('/',1)[1]] if '/by-hash/SHA256/' in path else paths[path]
            self.assertLessEqual(len(raw),limit)
            return raw
        signer=json.loads((folder/'repository-fixture.json').read_text())['signer']
        return files,fetch,signer

    def fixture(self,m,architecture='amd64'):
        files,fetch,signer=self.repository(m,architecture)
        with patch.object(m.material,'UBUNTU_ARCHIVE_SIGNER',signer):
            snapshot=m.material.RepositorySnapshot(architecture,'noble-updates',files['repository-keyring.gpg'],fetch)
            binding=snapshot.collect('libexample1','1.2-3+b1','main')
        objects={}
        def ref(raw):objects[sha(raw)]=raw;return {'sha256':sha(raw),'size':len(raw)}
        materials={name:ref(raw) for name,raw in snapshot.materials.items()}
        ref(files['libexample1_1.2-3+b1_amd64.deb.fixture'])
        control=ref(f'Package: libexample1\nVersion: 1.2-3+b1\nArchitecture: {architecture}\nSource: example (1.2-3)\n'.encode())
        recipes={r['id']:ref((ROOT/r['source_file']).read_bytes()) for r in m.RECIPES['sources']}
        config=(ROOT/'src-tauri/tauri.conf.json').read_bytes()
        inputs={'src-tauri/tauri.conf.json':ref(config)}
        generated=m.generated_files(json.loads(config),{name:objects[record['sha256']] for name,record in recipes.items()})
        records={name:{'kind':'generated-builder',**ref(raw)} for name,raw in generated.items()}
        supplier=runtime_elf();prefix=bytearray(supplier);prefix[352:368]=b'0123456789abcdef';launcher=b'launcher bytes'
        records['AppRun.wrapped']={'kind':'runtime-launcher',**ref(launcher)}
        own=b'\x7fELF independently built app'
        records['usr/bin/taggroove']={'kind':'application-build',**ref(own),'original':ref(own),
            'source_commit':'a'*40,'replay':{'steps':[],'sha256':sha(own),'size':len(own)},
            'observation':'builder-observed-full-byte-equality'}
        original=b'authenticated original member'
        records['usr/lib/example.so']={'kind':'distribution-file',**ref(original),'binding':0,
            'installed_path':'/usr/lib/example.so','requested_member':'usr/lib/example.so',
            'resolved_member':'usr/lib/example.so','original':ref(original),
            'replay':{'steps':[],'sha256':sha(original),'size':len(original)},
            'observation':'builder-observed-full-byte-equality'}
        runtime_arch='x86_64' if architecture=='amd64' else 'aarch64'
        selected_platform='linux-x64' if architecture=='amd64' else 'linux-arm64'
        runtime_spec={'asset_pins':{runtime_arch:{'runtime':{'sha256':sha(supplier),'byte_length':len(supplier)},
                                                   'launcher':{'sha256':sha(launcher),'byte_length':len(launcher)}}}}
        manifest={'schema':1,'source_commit':'a'*40,'version':'0.2.56','platform':selected_platform,
            'target':m.PLATFORMS[selected_platform][3],'appimage_sha256':sha(b'published image'),'appimage_size':15,
            'materials':materials,'bindings':[binding],'controls':[control],'files':records,
            'builder_recipes':recipes,'builder_inputs':inputs,'tool_pins':m.TOOLS['architectures'][architecture]['tools'],
            'runtime':{'supplier':ref(supplier),'prefix':ref(bytes(prefix)),
                       'mask':{'section':'.digest_md5','offset':352,'size':16}},
            'launcher':ref(launcher),'observations':'trusted-builder-extraction-and-full-byte-replay'}
        return manifest,objects,runtime_spec,signer

    @unittest.skipUnless(shutil.which('gpgv'), 'GnuPG signature verifier required')
    def test_two_target_export_replay_deduplicates_and_refuses_missing_extra_or_corrupt_objects(self):
        m=self.module();x64,xobjects,xspec,signer=self.fixture(m)
        self.assertTrue(callable(getattr(m,'export_capture',None)), 'Missing capture export')
        arm64,aobjects,aspec,_=self.fixture(m,'arm64')
        runtime_spec={'asset_pins':{**xspec['asset_pins'],**aspec['asset_pins']}}
        with tempfile.TemporaryDirectory() as folder,patch.object(m.material,'UBUNTU_ARCHIVE_SIGNER',signer):
            directory=Path(folder);outside=directory/'outside.txt';outside.write_bytes(b'preserved')
            m.export_capture(directory,x64,xobjects)
            with self.assertRaises(ValueError):m.source_files(directory,'a'*40,'0.2.56',runtime_spec)
            m.export_capture(directory,arm64,aobjects)
            self.assertEqual(outside.read_bytes(),b'preserved')
            selected=m.source_files(directory,'a'*40,'0.2.56',runtime_spec)
            self.assertEqual({name for name in selected if '/targets/' in name},
                             {'linux-distribution/targets/linux-x64.json','linux-distribution/targets/linux-arm64.json'})
            self.assertEqual(len([name for name in selected if '/objects/' in name]),len(set(xobjects)|set(aobjects)))
            extra=b'unbound';m.export_capture(directory,x64,{**xobjects,sha(extra):extra})
            with self.assertRaises(ValueError):m.source_files(directory,'a'*40,'0.2.56',runtime_spec)
            (directory/'objects'/sha(extra)).unlink()
            digest=next(iter(xobjects));(directory/'objects'/digest).write_bytes(b'corrupt')
            with self.assertRaises(ValueError):m.source_files(directory,'a'*40,'0.2.56',runtime_spec)

    def test_recipe_rpath_is_derived_from_packaged_destination_and_file_role(self):
        m=self.module()
        self.assertTrue(callable(getattr(m,'expected_rpath',None)), 'Missing recipe RPATH derivation')
        self.assertEqual(m.expected_rpath('usr/lib/libexample.so'),'$ORIGIN')
        self.assertEqual(m.expected_rpath('usr/bin/taggroove'),'$ORIGIN/../lib')
        self.assertEqual(m.expected_rpath('usr/lib/gio/modules/libexample.so'),'$ORIGIN')
        self.assertEqual(m.expected_rpath('usr/lib/x86_64-linux-gnu/webkit2gtk-4.1/WebKitWebProcess'),'$ORIGIN')
        original=elf_with_rpath('/opt/vendor:$ORIGIN/extra')
        self.assertEqual(m.expected_rpath('usr/lib/gio/modules/libexample.so',original),
                         '/opt/vendor:$ORIGIN/extra:$ORIGIN')
        self.assertEqual(m.expected_rpath('usr/lib/libexample.so',original),'$ORIGIN')
        self.assertEqual(m.expected_rpath('usr/lib/nested/libwebkit.so',elf_with_rpath('/usr/vendor')),
                         '././/vendor:$ORIGIN')

    def test_copyright_alias_lookup_reaches_the_package_owning_the_canonical_document(self):
        m=self.module()
        alias={'path':'/usr/share/doc/alias/copyright','canonical':'/usr/share/doc/base/copyright',
               'package':'alias','version':'1'}
        document={'path':'/usr/share/doc/base/copyright','canonical':'/usr/share/doc/base/copyright',
                  'package':'base','version':'1'}
        found=m.candidates({alias['path']:[alias],document['path']:[document]},
                           'usr/share/doc/alias/copyright','x86_64-linux-gnu')
        self.assertEqual({row['package'] for row in found},{'alias','base'})

    @unittest.skipUnless(shutil.which('gpgv'), 'GnuPG signature verifier required')
    def test_capture_qualifies_actual_file_bytes_and_refuses_unknown_colliding_or_wrong_members(self):
        m=self.module();base,objects,runtime_spec,signer=self.fixture(m)
        self.assertTrue(callable(getattr(m,'inspect_image',None)), 'Missing capture inspection boundary')
        raw_files={name:objects[record['sha256']] for name,record in base['files'].items()}
        entries={name:{'kind':'file','size':len(raw),'mode':'-rw-r--r--'} for name,raw in raw_files.items()}
        original=raw_files['usr/lib/example.so']
        candidate={'path':'/usr/lib/example.so','canonical':'/usr/lib/example.so','package':'libexample1',
                   'version':'1.2-3+b1','architecture':'amd64','source_package':'example','source_version':'1.2-3'}
        index={'/usr/lib/example.so':[candidate]}
        source,fetch,_=self.repository(m)
        with tempfile.TemporaryDirectory() as folder,patch.object(m.material,'UBUNTU_ARCHIVE_SIGNER',signer):
            root=Path(folder);image=root/'image.AppImage';image.write_bytes(b'observed image bytes')
            for row in m.RECIPES['sources']:
                path=root/row['source_file'];path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes((ROOT/row['source_file']).read_bytes())
            for name in ['src-tauri/tauri.conf.json','src/licenses/appimage-common/SOURCES.json']:
                path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes((ROOT/name).read_bytes())
            path=root/'scripts/notices/linux-runtime-material.json';path.parent.mkdir(parents=True,exist_ok=True);path.write_text(json.dumps(runtime_spec))
            runtime_path=root/'target/linux-source-tools/amd64/runtime';runtime_path.parent.mkdir(parents=True);runtime_path.write_bytes(runtime_elf())
            own=root/'target/x86_64-unknown-linux-gnu/release/taggroove';own.parent.mkdir(parents=True);own.write_bytes(raw_files['usr/bin/taggroove'])
            def snapshots(_architecture):
                return [m.material.RepositorySnapshot('amd64','noble-updates',source['repository-keyring.gpg'],fetch)]
            def members(_raw,_selected,requested):return {name:{'path':name,'bytes':original} for name in requested}
            with patch.object(m.host,'system',return_value='Linux'), \
                 patch.object(m,'inspect_image',return_value=(objects[base['runtime']['prefix']['sha256']],entries,raw_files)), \
                 patch.object(m,'installed_inventory',return_value=index), \
                 patch.object(m,'read_installed',return_value=original), \
                 patch.object(m,'repository_snapshots',side_effect=snapshots), \
                 patch.object(m,'package_control',return_value=objects[base['controls'][0]['sha256']]), \
                 patch.object(m.packages,'package_members',side_effect=members):
                manifest,captured=m.capture(root,'linux-x64','a'*40,'0.2.56',image,Path('inspector'),{},base['tool_pins'])
                self.assertEqual(manifest['files']['usr/lib/example.so']['kind'],'distribution-file')
                self.assertEqual(manifest['appimage_sha256'],sha(image.read_bytes()))
                self.assertEqual(m.verify_materials(manifest,captured,runtime_spec,'a'*40,'0.2.56','linux-x64'),set(captured))
                module_path='usr/lib/gio/modules/cache-module.so';cache_path='usr/lib/gio/modules/giomodule.cache'
                for name,body in [(module_path,original),(cache_path,b'generated cache')]:
                    entries[name]={'kind':'file','size':len(body),'mode':'-rw-r--r--'};raw_files[name]=body
                for name in ['/usr/lib/x86_64-linux-gnu/gio/modules/cache-module.so',
                             '/usr/lib/x86_64-linux-gnu/glib-2.0/gio-querymodules']:
                    index[name]=[{**candidate,'path':name,'canonical':name}]
                with_cache,cache_objects=m.capture(root,'linux-x64','a'*40,'0.2.56',image,Path('inspector'),{},base['tool_pins'])
                cache=with_cache['files'][cache_path]
                self.assertEqual(cache['inputs'],{module_path:sha(original)})
                self.assertEqual(cache['generator']['requested_member'],
                                 'usr/lib/x86_64-linux-gnu/glib-2.0/gio-querymodules')
                self.assertEqual(m.verify_materials(with_cache,cache_objects,runtime_spec,'a'*40,'0.2.56','linux-x64'),set(cache_objects))
                unknown='usr/share/unbound';entries[unknown]={'kind':'file','size':3,'mode':'-rw-r--r--'};raw_files[unknown]=b'bad'
                with self.assertRaises(ValueError):m.capture(root,'linux-x64','a'*40,'0.2.56',image,Path('inspector'),{},base['tool_pins'])
                entries.pop(unknown);raw_files.pop(unknown)
                with patch.object(m.packages,'package_members',return_value={'usr/lib/example.so':{'path':'usr/lib/example.so','bytes':b'wrong'}}),self.assertRaises(ValueError):
                    m.capture(root,'linux-x64','a'*40,'0.2.56',image,Path('inspector'),{},base['tool_pins'])
                alias={**candidate,'path':'/usr/other/example.so','canonical':'/usr/other/example.so'};index[alias['path']]=[alias]
                with self.assertRaises(ValueError):m.capture(root,'linux-x64','a'*40,'0.2.56',image,Path('inspector'),{},base['tool_pins'])

    @unittest.skipUnless(shutil.which('gpgv'), 'GnuPG signature verifier required')
    def test_offline_verification_reauthenticates_source_and_refuses_unbound_or_changed_objects(self):
        m=self.module();manifest,objects,runtime_spec,signer=self.fixture(m)
        with patch.object(m.material,'UBUNTU_ARCHIVE_SIGNER',signer), \
             patch.object(m.material,'download',side_effect=AssertionError('Network access')), \
             patch.object(m.packages,'package_members',side_effect=AssertionError('Package tool execution')), \
             patch.object(m.packages,'replay_packaging',side_effect=AssertionError('ELF tool execution')):
            required=m.verify_materials(manifest,objects,runtime_spec,'a'*40,'0.2.56','linux-x64')
            self.assertEqual(required,set(objects))
            for digest in required:
                changed=dict(objects);changed[digest] += b'tampered'
                with self.subTest(digest=digest),self.assertRaises(ValueError):
                    m.verify_materials(manifest,changed,runtime_spec,'a'*40,'0.2.56','linux-x64')
            altered=copy.deepcopy(manifest);altered['bindings'][0]['source']['version']='1.2-unknown'
            with self.assertRaises(ValueError):m.verify_materials(altered,objects,runtime_spec,'a'*40,'0.2.56','linux-x64')
            altered=copy.deepcopy(manifest);altered['files']['unknown']={'kind':'unbound','sha256':sha(b'bad'),'size':3}
            with self.assertRaises(ValueError):m.verify_materials(altered,{**objects,sha(b'bad'):b'bad'},runtime_spec,'a'*40,'0.2.56','linux-x64')
            with self.assertRaises(ValueError):m.verify_materials(manifest,objects,runtime_spec,'b'*40,'0.2.56','linux-x64')
            with self.assertRaises(ValueError):m.verify_materials(manifest,objects,runtime_spec,'a'*40,'0.2.57','linux-x64')
            with self.assertRaises(ValueError):m.verify_materials(manifest,objects,runtime_spec,'a'*40,'0.2.56','linux-arm64')

    @unittest.skipUnless(shutil.which('gpgv'), 'GnuPG signature verifier required')
    def test_cache_provenance_requires_the_complete_verified_input_set_and_generator(self):
        m=self.module();manifest,objects,runtime_spec,signer=self.fixture(m)
        raw=b'generated cache bytes';objects[sha(raw)]=raw
        path='usr/lib/gio/modules/giomodule.cache'
        manifest['files'][path]={'kind':'generated-data','sha256':sha(raw),'size':len(raw),
            'inputs':{},'generator':{'binding':0,'requested_member':'usr/bin/gio-querymodules',
                'resolved_member':'usr/bin/gio-querymodules','original':{'sha256':sha(b'tool'),'size':4}},
            'observation':'builder-observed-cache-bytes; recipe-derived-inputs; cache regeneration not claimed'}
        with patch.object(m.material,'UBUNTU_ARCHIVE_SIGNER',signer),self.assertRaises(ValueError):
            m.verify_materials(manifest,{**objects,sha(b'tool'):b'tool'},runtime_spec,'a'*40,'0.2.56','linux-x64')
        manifest['files']['usr/lib/gio/modules/example.so']=manifest['files'].pop('usr/lib/example.so')
        record=manifest['files'][path];record['inputs']={'usr/lib/gio/modules/example.so':sha(b'authenticated original member')}
        objects[sha(b'tool')]=b'tool'
        with patch.object(m.material,'UBUNTU_ARCHIVE_SIGNER',signer):
            m.verify_materials(manifest,objects,runtime_spec,'a'*40,'0.2.56','linux-x64')
            record['generator']['requested_member']='usr/bin/unrelated-tool'
            with self.assertRaises(ValueError):m.verify_materials(manifest,objects,runtime_spec,'a'*40,'0.2.56','linux-x64')

    @unittest.skipUnless(shutil.which('gpgv'), 'GnuPG signature verifier required')
    def test_unknown_aliases_and_directories_cannot_hide_unbound_payloads(self):
        m=self.module();manifest,objects,runtime_spec,signer=self.fixture(m)
        raw=b'usr/bin/taggroove';objects[sha(raw)]=raw
        manifest['files']['.DirIcon']={'kind':'symlink','sha256':sha(raw),'size':len(raw)}
        with patch.object(m.material,'UBUNTU_ARCHIVE_SIGNER',signer),self.assertRaises(ValueError):
            m.verify_materials(manifest,objects,runtime_spec,'a'*40,'0.2.56','linux-x64')
        manifest['files'].pop('.DirIcon');manifest['files']['unknown']={'kind':'directory','size':3,'sha256':sha(raw)}
        with patch.object(m.material,'UBUNTU_ARCHIVE_SIGNER',signer),self.assertRaises(ValueError):
            m.verify_materials(manifest,objects,runtime_spec,'a'*40,'0.2.56','linux-x64')

    def test_capture_refuses_non_linux_execution(self):
        m=self.module()
        with patch.object(m.host,'system',return_value='Darwin'),self.assertRaises(ValueError):
            m.capture(ROOT,'linux-x64','a'*40,'0.2.56',Path('image'),Path('7zz'),{}, {})

    def test_export_preserves_object_or_manifest_arriving_before_publication(self):
        m=self.module();raw=b'new captured object';digest=sha(raw)
        for arrival in ['object','new-manifest','changed-manifest']:
            with self.subTest(arrival=arrival),tempfile.TemporaryDirectory() as folder:
                directory=Path(folder);manifest={'platform':'linux-x64'}
                path=directory/('objects/'+digest if arrival=='object' else 'linux-source-linux-x64.json')
                if arrival=='changed-manifest':path.write_bytes(b'observed manifest')
                original=m.core.publish_outputs
                def arrive(root,outputs,guards=None):
                    path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b'outside arrival')
                    return original(root,outputs,guards)
                with patch.object(m.core,'publish_outputs',side_effect=arrive),self.assertRaises(ValueError):
                    m.export_capture(directory,manifest,{digest:raw})
                self.assertEqual(path.read_bytes(),b'outside arrival')

    def test_cached_runtime_checks_its_pin_size_before_reading(self):
        m=self.module()
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);path=root/'target/linux-source-tools/amd64/runtime'
            path.parent.mkdir(parents=True);path.write_bytes(b'too large')
            with patch.object(Path,'open',side_effect=AssertionError('Oversized cache was read')),self.assertRaises(ValueError):
                m.pinned_runtime(root,'amd64',{'byte_length':3,'sha256':sha(b'abc')})

    def test_runtime_inspector_objects_and_manifests_bound_reads_even_if_files_grow(self):
        m=self.module();original_open=Path.open
        for kind in ['runtime','inspector','object','manifest']:
            with self.subTest(kind=kind),tempfile.TemporaryDirectory() as folder:
                root=Path(folder);(root/'objects').mkdir();raw=b'abc'
                if kind=='runtime':path=root/'target/linux-source-tools/amd64/runtime'
                elif kind=='inspector':path=root/'inspector'
                elif kind=='object':path=root/'objects'/sha(raw)
                else:path=root/'linux-source-linux-x64.json'
                path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
                class Growing(io.BytesIO):
                    def read(self,size=-1):
                        if not 0<size<=len(raw)+1:raise AssertionError('Unbounded file read after observed stat')
                        return super().read(size)
                def opened(candidate,*args,**kwargs):
                    if candidate==path:return Growing(raw+b'x'*1000)
                    return original_open(candidate,*args,**kwargs)
                with patch.object(Path,'open',new=opened),self.assertRaises(ValueError):
                    if kind=='runtime':m.pinned_runtime(root,'amd64',{'byte_length':len(raw),'sha256':sha(raw)})
                    elif kind=='inspector':m.inspect_image(Path('unused-image'),path,'amd64')
                    else:m.source_files(root,'a'*40,'0.2.56',{})


if __name__=='__main__':unittest.main()
