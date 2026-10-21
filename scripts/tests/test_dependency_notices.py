"""Focused source-bound notice collector tests; no application builds or network."""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
import subprocess
import sys
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "dependency-notices.py"


class DependencyNotices(unittest.TestCase):
    def test_legacy_archive_supplement_requires_exact_source_header_without_claiming_a_vcs_commit(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            raw = b'// Copyright Example. Licensed under Apache-2.0.\n'
            archive = root / 'legacy.crate'
            with tarfile.open(archive, 'w:gz') as handle:
                member = tarfile.TarInfo('legacy-1.0.0/lib.rs')
                member.size = len(raw)
                handle.addfile(member, io.BytesIO(raw))
            (root / 'header.txt').write_bytes(raw)
            checksum = m.sha(archive.read_bytes())
            item = {'path':'lib.rs.header','source_file':'header.txt','sha256':m.sha(raw),
                    'archive_member':'legacy-1.0.0/lib.rs','header_prefix_bytes':len(raw)}
            supplements = {'cargo:legacy@1.0.0': {'archive_sha256':checksum,'archive_bound':True,'materials':[item]}}
            source = {'vcs':None}
            result = m.supplemental_materials(root,'cargo:legacy@1.0.0',checksum,source,archive,supplements,{})
            self.assertEqual(result[0]['text'], raw.decode())
            supplements['cargo:legacy@1.0.0']['materials'] = [{k:v for k,v in item.items() if k not in ('archive_member','header_prefix_bytes')}]
            with self.assertRaisesRegex(ValueError, 'header'):
                m.supplemental_materials(root,'cargo:legacy@1.0.0',checksum,source,archive,supplements,{})
            supplements['cargo:legacy@1.0.0']['materials'] = [item]
            item['header_prefix_bytes'] -= 1
            with self.assertRaisesRegex(ValueError, 'header differs'):
                m.supplemental_materials(root,'cargo:legacy@1.0.0',checksum,source,archive,supplements,{})

    def test_rendered_payload_has_one_final_newline_and_preserves_source_text(self):
        m = self.module()
        raw = 'Copyright Example\r\nPermission text.\r\n\r\n'
        data = {'components': [], 'texts': {'a' * 64: raw}}
        rendered = m.render_notices(data)
        self.assertTrue(rendered.endswith('Permission text.\n'))
        self.assertFalse(rendered.endswith('\n\n'))
        self.assertEqual(data['texts']['a' * 64], raw)

    def module(self):
        self.assertTrue(SCRIPT.is_file(), "The source-bound dependency notice collector is missing")
        spec = importlib.util.spec_from_file_location("dependency_notices", SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_runtime_closure_excludes_dev_build_and_macro_only_dependencies(self):
        m = self.module()
        def package(name, macro=False):
            return {"id": name, "name": "opentag" if name == "app" else name, "version": "1.0.0", "source": None if name == "app" else "registry+https://github.com/rust-lang/crates.io-index", "targets": [{"kind": ["proc-macro" if macro else "lib"]}]}
        def node(name, *deps):
            return {"id": name, "deps": [{"pkg": pkg, "dep_kinds": [{"kind": kind}]} for pkg, kind in deps]}
        data={"packages":[package(n,n=='macro') for n in ['app','normal','dev','build','macro','macro_only','shared']],"resolve":{"nodes":[
            node('app',('normal',None),('macro',None),('dev','dev'),('build','build')),
            node('normal',('shared',None)),node('macro',('macro_only',None),('shared',None)),
            *[node(n) for n in ['dev','build','macro_only','shared']],
        ]}}
        selected=m.runtime_packages(data)
        self.assertEqual({p['name'] for p in selected}, {'normal','shared'})
        data['resolve']['nodes'][0]['deps'].append({'pkg':'missing','dep_kinds':[{'kind':None}]})
        with self.assertRaisesRegex(ValueError,'Missing'):
            m.runtime_packages(data)

    def test_archive_material_is_bound_to_exact_archive_and_nested_members(self):
        m=self.module()
        with tempfile.TemporaryDirectory() as temporary:
            archive=Path(temporary)/'example-1.0.0.crate'
            data={'example-1.0.0/Cargo.toml':b'[package]\nname="example"\nversion="1.0.0"\nlicense="MIT"\n',
                  'example-1.0.0/LICENSE':b'Copyright Example\r\nPermission text.\r\n',
                  'example-1.0.0/UNLICENSE':b'Public-domain dedication.\n',
                  'example-1.0.0/nested/NOTICE': 'Nested copyright ©\n'.encode(),
                  'example-1.0.0/src/lib.rs': b'pub fn example() {}'}
            with tarfile.open(archive,'w:gz') as handle:
                for path,raw in data.items():
                    item=tarfile.TarInfo(path);item.size=len(raw);handle.addfile(item,io.BytesIO(raw))
            checksum=hashlib.sha256(archive.read_bytes()).hexdigest()
            result=m.archive_materials(archive,checksum,'example-1.0.0')
            self.assertEqual(result['package']['name'],'example')
            self.assertEqual({x['path'] for x in result['materials']},{'LICENSE','UNLICENSE','nested/NOTICE'})
            texts={x['path']:x['text'] for x in result['materials']}
            self.assertEqual(texts['LICENSE'],'Copyright Example\r\nPermission text.\r\n')
            for item in result['materials']:
                self.assertEqual(hashlib.sha256(item['text'].encode()).hexdigest(),item['sha256'])
            with self.assertRaisesRegex(ValueError,'checksum'):
                m.archive_materials(archive,'0'*64,'example-1.0.0')

    def test_archive_unsafe_paths_duplicate_members_and_license_links_are_refused(self):
        m=self.module()
        for mode in ['traversal','duplicate','symlink']:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary:
                archive=Path(temporary)/'bad.crate'
                with tarfile.open(archive,'w:gz') as handle:
                    item=tarfile.TarInfo('example-1.0.0/Cargo.toml');raw=b'[package]\nname="example"\nversion="1.0.0"\n';item.size=len(raw);handle.addfile(item,io.BytesIO(raw))
                    name='example-1.0.0/../LICENSE' if mode=='traversal' else 'example-1.0.0/LICENSE'
                    item=tarfile.TarInfo(name)
                    if mode=='symlink':item.type=tarfile.SYMTYPE;item.linkname='/outside/NOTICE';handle.addfile(item)
                    else:
                        raw=b'License';item.size=len(raw);handle.addfile(item,io.BytesIO(raw))
                        if mode=='duplicate':handle.addfile(item,io.BytesIO(raw))
                with self.assertRaises(ValueError):
                    m.archive_materials(archive,hashlib.sha256(archive.read_bytes()).hexdigest(),'example-1.0.0')

    def fixture(self,root):
        (root/'src-tauri').mkdir()
        (root/'Cargo.toml').write_text('[workspace]\nmembers=["src-tauri"]\n')
        (root/'src-tauri/Cargo.toml').write_text('[package]\nname="opentag"\nversion="0.2.0"\n[dependencies]\nexample="1"\n')
        (root/'Cargo.lock').write_text('version=4\n[[package]]\nname="opentag"\nversion="0.2.0"\n[[package]]\nname="example"\nversion="1.0.0"\nsource="registry+https://github.com/rust-lang/crates.io-index"\nchecksum="'+'a'*64+'"\n')
        (root/'package.json').write_text(json.dumps({'name':'taggroove','version':'0.2.0','dependencies':{}}))
        (root/'package-lock.json').write_text(json.dumps({'name':'taggroove','version':'0.2.0','lockfileVersion':3,'packages':{'':{'name':'taggroove','version':'0.2.0'}}}))

    def test_source_fingerprints_ignore_only_release_stamps_and_detect_dependency_changes(self):
        m=self.module()
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);self.fixture(root);before=m.source_fingerprints(root)
            for name in ['src-tauri/Cargo.toml','Cargo.lock','package.json','package-lock.json']:
                path=root/name;path.write_text(path.read_text().replace('0.2.0','0.2.99'))
            self.assertEqual(m.source_fingerprints(root),before)
            path=root/'src-tauri/Cargo.toml';path.write_text(path.read_text().replace('example="1"','example="2"'))
            self.assertNotEqual(m.source_fingerprints(root),before)

    def test_runtime_npm_closure_and_helper_provider_roles_are_separate(self):
        m=self.module()
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);self.fixture(root)
            manifest={'name':'taggroove','dependencies':{'@scope/runtime':'1'},'devDependencies':{'vite':'7'}}
            packages={'':{},'node_modules/@scope/runtime':{'version':'1.0.0','dependencies':{'child':'1'}},
                      'node_modules/@scope/runtime/node_modules/child':{'version':'1.0.0'},
                      'node_modules/vite':{'version':'7.0.0','dependencies':{'helper_dependency':'1'}},
                      'node_modules/helper_dependency':{'version':'1.0.0'}}
            (root/'package.json').write_text(json.dumps(manifest));(root/'package-lock.json').write_text(json.dumps({'packages':packages}))
            found=m.npm_runtime_entries(root)
            self.assertEqual(set(found),{'node_modules/@scope/runtime','node_modules/@scope/runtime/node_modules/child','node_modules/vite'})
            # If a tool is explicitly a runtime dependency, traverse its normal
            # dependencies too, even when it was first seen as a helper provider.
            manifest['dependencies']['vite']='7';(root/'package.json').write_text(json.dumps(manifest))
            self.assertIn('node_modules/helper_dependency',m.npm_runtime_entries(root))

    def test_source_input_fingerprints_are_portable_across_checkout_newlines(self):
        m=self.module()
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);self.fixture(root)
            for name,text in {'scripts/notices/supplements.json':'{\n"schema":1,"components":{}\n}\n',
                              '.cargo/config.toml':'[build]\njobs=1\n',
                              'scripts/dependency-notices.py':'# fixture generator\nprint("hello")\n'}.items():
                path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text(text)
            before=m.source_fingerprints(root)
            for path in root.rglob('*'):
                if path.is_file():path.write_bytes(path.read_bytes().replace(b'\n',b'\r\n'))
            self.assertEqual(m.source_fingerprints(root),before)

    def test_inventory_requires_all_target_members_exact_material_and_current_source(self):
        m=self.module()
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);self.fixture(root)
            text='Exact copyright ©\r\n';sha=hashlib.sha256(text.encode()).hexdigest()
            record={'id':'cargo:example@1.0.0','ecosystem':'cargo','name':'example','version':'1.0.0','archive_sha256':'a'*64,'license':'MIT','targets':sorted(m.TARGETS),'materials':[{'path':'LICENSE','sha256':sha}]}
            inventory={'schema':1,'inputs':m.source_fingerprints(root),'features':['tauri/custom-protocol'],'targets':{name:[record['id']] for name in m.TARGETS},'components':[record],'texts':{sha:text}}
            m.validate_inventory(root,inventory)
            def altered(change):
                value=json.loads(json.dumps(inventory));change(value)
                with self.assertRaises(ValueError):m.validate_inventory(root,value)
            altered(lambda v:v['components'].clear())
            altered(lambda v:v['components'].append(v['components'][0]))
            altered(lambda v:v['texts'].__setitem__(sha,'Changed notice'))
            altered(lambda v:v['components'][0]['materials'].clear())
            altered(lambda v:v['components'][0].__setitem__('archive_sha256','b'*64))
            altered(lambda v:v['targets'][next(iter(m.TARGETS))].clear())
            (root/'package.json').write_text('{"name":"taggroove","dependencies":{"new":"1"}}')
            with self.assertRaisesRegex(ValueError,'source|inputs'):
                m.validate_inventory(root,inventory)

    def test_generate_check_and_material_replay_require_attached_untampered_payloads(self):
        m=self.module()
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);self.fixture(root)
            cache=root/'cache';cache.mkdir();metadata=root/'metadata';metadata.mkdir();npm=root/'npm';npm.mkdir()
            archive=cache/'example-1.0.0.crate'
            with tarfile.open(archive,'w:gz') as handle:
                for name,raw in {'Cargo.toml':b'[package]\nname="example"\nversion="1.0.0"\nlicense="MIT"\n','LICENSE':'Copyright © Example\nPermission retained.\n'.encode()}.items():
                    item=tarfile.TarInfo('example-1.0.0/'+name);item.size=len(raw);handle.addfile(item,io.BytesIO(raw))
            checksum=hashlib.sha256(archive.read_bytes()).hexdigest();lock=root/'Cargo.lock';lock.write_text(lock.read_text().replace('a'*64,checksum))
            data={'packages':[{'id':'app','name':'opentag','version':'0.2.0','source':None,'targets':[{'kind':['lib']}]},{'id':'example','name':'example','version':'1.0.0','source':'registry+https://github.com/rust-lang/crates.io-index','targets':[{'kind':['lib']}]}],
                  'resolve':{'nodes':[{'id':'app','deps':[{'pkg':'example','dep_kinds':[{'kind':None}]}]},{'id':'example','deps':[]}]}}
            for platform,target in m.TARGETS.items():
                (metadata/(platform+'.json')).write_text(json.dumps({'schema':1,'platform':platform,'target':target,'features':m.FEATURES,'inputs':m.graph_inputs(root),'metadata':data}))
            (root/'THIRD_PARTY_NOTICES.md').write_text('# Application notices\n\nKeep existing text.\n')
            def run(*args):
                return subprocess.run([sys.executable,str(SCRIPT),'--root',str(root),*args],capture_output=True,text=True)
            generated=run('--generate','--metadata-directory',str(metadata),'--cargo-cache',str(cache),'--npm-cache',str(npm))
            self.assertEqual(generated.returncode,0,generated.stderr)
            checked=run('--check','--verify-material','--cargo-cache',str(cache),'--npm-cache',str(npm),'--metadata-directory',str(metadata))
            self.assertEqual(checked.returncode,0,checked.stderr)
            self.assertTrue((root/'THIRD_PARTY_DEPENDENCIES.md').is_file(),'Generator did not publish the notice payload')
            payload=(root/'THIRD_PARTY_DEPENDENCIES.md').read_text();common=(root/'THIRD_PARTY_NOTICES.md').read_text()
            self.assertIn(payload.strip(),common);self.assertIn('Keep existing text.',common)
            # Reusing snapshots after changing Rust dependencies must not make a
            # new inventory appear current while it still omits the new graph.
            cargo=root/'src-tauri/Cargo.toml';original=cargo.read_bytes();inventory_before=(root/m.INVENTORY).read_bytes()
            cargo.write_bytes(original.replace(b'example="1"',b'example="2"'))
            stale=run('--generate','--metadata-directory',str(metadata),'--cargo-cache',str(cache),'--npm-cache',str(npm))
            self.assertNotEqual(stale.returncode,0,'Stale metadata snapshots must not authorize a regenerated inventory')
            self.assertEqual((root/m.INVENTORY).read_bytes(),inventory_before)
            cargo.write_bytes(original)
            (root/'THIRD_PARTY_NOTICES.md').write_text('# Detached common notices\n')
            self.assertNotEqual(run('--check').returncode,0)
            (root/'THIRD_PARTY_NOTICES.md').write_text(common)
            archive_before=archive.read_bytes()
            with archive.open('ab') as handle:handle.write(b'changed archive')
            self.assertNotEqual(run('--check','--verify-material','--cargo-cache',str(cache),'--npm-cache',str(npm)).returncode,0)
            archive.write_bytes(archive_before)
            (root/m.INVENTORY).write_bytes(b'old inventory')
            (root/m.PAYLOAD).write_bytes(b'old payload')
            (root/'THIRD_PARTY_NOTICES.rtf').unlink();(root/'THIRD_PARTY_NOTICES.rtf').mkdir()
            refusal=run('--generate','--metadata-directory',str(metadata),'--cargo-cache',str(cache),'--npm-cache',str(npm))
            self.assertNotEqual(refusal.returncode,0)
            self.assertEqual((root/m.INVENTORY).read_bytes(),b'old inventory','a late output refusal must preserve every earlier file')
            self.assertEqual((root/m.PAYLOAD).read_bytes(),b'old payload')
            self.assertEqual((root/'THIRD_PARTY_NOTICES.md').read_text(),common)

    def test_handled_mid_publication_failure_restores_all_original_outputs(self):
        m=self.module()
        self.assertTrue(callable(getattr(m,'publish_outputs',None)),'Missing staged publication with rollback')
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);(root/'first').write_bytes(b'original first');(root/'second').write_bytes(b'original second')
            native_replace=m.os.replace;calls=0
            def fail_second(source,target):
                nonlocal calls
                calls+=1
                if calls==2:raise OSError('injected second-publication failure')
                return native_replace(source,target)
            with patch.object(m.os,'replace',side_effect=fail_second):
                with self.assertRaisesRegex(OSError,'second-publication'):
                    m.publish_outputs(root,{'first':b'new first','second':b'new second','absent':b'new absent'})
            self.assertEqual((root/'first').read_bytes(),b'original first')
            self.assertEqual((root/'second').read_bytes(),b'original second')
            self.assertFalse((root/'absent').exists())

    def test_unobservable_published_output_retains_its_original_recovery_copy(self):
        m=self.module()
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary);first=root/'first';first.write_bytes(b'original first')
            native_read=Path.read_bytes
            def unreadable_after_move(path):
                raw=native_read(path)
                if path==first and raw==b'new first':raise OSError('readback unavailable')
                return raw
            with patch.object(Path,'read_bytes',unreadable_after_move):
                with self.assertRaises((OSError,ValueError)):
                    m.publish_outputs(root,{'first':b'new first'})
            self.assertTrue(any(path.read_bytes()==b'original first' for path in root.glob('.dependency-notices-*')),
                            'unobservable post-move state must retain the old source for recovery')


if __name__=='__main__':
    unittest.main()
