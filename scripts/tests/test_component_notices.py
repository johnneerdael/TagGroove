"""Focused native/installer notice and actual package binding checks; no target execution."""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
import zipfile
import tarfile
import subprocess
import sys

SCRIPT = Path(__file__).resolve().parents[1] / 'component-notices.py'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


class ComponentNoticesTests(unittest.TestCase):
    def module(self):
        self.assertTrue(SCRIPT.is_file(), 'Missing native/installer notice material checker')
        spec = importlib.util.spec_from_file_location('component_notices', SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_notice_slices_require_original_source_and_exact_boundaries(self):
        m = self.module()
        original = b'/* Copyright example. Permission text. */\nint implementation;\n'
        length = original.index(b'int implementation;')
        entry = {'source_sha256': sha(original), 'offset': 0, 'length': length,
                 'sha256': sha(original[:length])}
        self.assertEqual(m.notice_slice(original, entry), original[:length])
        for field, value in [('source_sha256', '0'*64), ('offset', -1), ('length', len(original)+1), ('sha256', '0'*64)]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                m.notice_slice(original, dict(entry, **{field: value}))

    def test_leading_notices_include_second_c_comment_and_assembly_header_only(self):
        m = self.module()
        c = b'/* Ogg support */\n\n/* Copyright supplier. Permission granted. */\n\n#include "header.h"\n'
        self.assertEqual(m.leading_notices(c), c[:c.index(b'#include')])
        assembly = b';* Copyright supplier\n;* Permission granted\n\n%define CODE 1\n'
        self.assertEqual(m.leading_notices(assembly), assembly[:assembly.index(b'%define')])
        with self.assertRaises(ValueError):
            m.leading_notices(b'#include "other.c"\n')

    def test_android_runtime_requires_exact_package_and_sdk_bytes_and_notice_material(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            sdk = root/'ndk'
            host = sdk/'toolchains/llvm/prebuilt/test-host'
            lib = host/'sysroot/usr/lib/aarch64-linux-android/libc++_shared.so'
            lib.parent.mkdir(parents=True)
            library = b'known native runtime'
            lib.write_bytes(library)
            (sdk/'source.properties').write_text('Pkg.Revision = 28.2.13676358\n')
            notice = b'Exact LLVM runtime notice\n'
            (host/'NOTICE').write_bytes(notice)
            (root/'notice.txt').write_bytes(notice)
            spec = {'android': {'ndk_revision':'28.2.13676358', 'runtime_sha256':sha(library),
                               'notice_sources':['notice.txt']}}
            apk = root/'app.apk'
            def package(runtime=library, extra=False):
                with zipfile.ZipFile(apk,'w') as z:
                    z.writestr('lib/arm64-v8a/libopentag_lib.so', b'app source')
                    z.writestr('lib/arm64-v8a/libc++_shared.so', runtime)
                    if extra: z.writestr('lib/arm64-v8a/libunmapped.so',b'unknown')
            package()
            m.verify_android(root,spec,apk,sdk)
            for kind in ['changed', 'extra', 'notice', 'revision']:
                with self.subTest(kind=kind):
                    package(runtime=b'changed' if kind=='changed' else library, extra=kind=='extra')
                    (host/'NOTICE').write_bytes(b'unrelated' if kind=='notice' else notice)
                    (sdk/'source.properties').write_text('Pkg.Revision = '+('29' if kind=='revision' else '28.2.13676358')+'\n')
                    with self.assertRaises(ValueError):
                        m.verify_android(root,spec,apk,sdk)

    def test_compiled_source_notices_cannot_omit_a_distinct_permission_header(self):
        m = self.module()
        lgpl = b'/*\n * GNU Lesser\n * General Public License\n */\nint generic;\n'
        permissive = b'/* Copyright supplier. Permission is hereby granted. */\nint helper;\n'
        header = m.leading_notices(permissive)
        stream = io.BytesIO()
        with tarfile.open(fileobj=stream,mode='w') as a:
            for name,raw in [('libavutil/generic.c',lgpl),('libavutil/helper.c',permissive)]:
                item=tarfile.TarInfo('ffmpeg-test/'+name);item.size=len(raw);a.addfile(item,io.BytesIO(raw))
        raw = stream.getvalue()
        spec = {'bindings':{'ffmpeg':{'sha256':sha(raw)}},'materials':{}}
        expected = {'libavutil/generic.c','libavutil/helper.c'}
        with self.assertRaisesRegex(ValueError,'Unmapped'):
            m.verify_ffmpeg_coverage(spec,raw,'ffmpeg-test',expected,lambda _path:header)
        spec['materials']['helper']={'source_file':'notice.txt','sha256':sha(header),'origin':{
            'kind':'tar_slice','member':'ffmpeg-test/libavutil/helper.c','source_sha256':sha(permissive),
            'offset':0,'length':len(header),'sha256':sha(header)}}
        m.verify_ffmpeg_coverage(spec,raw,'ffmpeg-test',expected,lambda _path:header)
        with self.assertRaisesRegex(ValueError,'notice'):
            m.verify_ffmpeg_coverage(spec,raw,'ffmpeg-test',expected,lambda _path:b'changed')

    def test_windows_helper_inventory_rejects_extra_and_changed_binaries(self):
        m = self.module()
        expected={'$PLUGINSDIR/System.dll':b'actual helper'}
        spec={'windows':{'helpers':{name:{'sha256':sha(raw)} for name,raw in expected.items()}}}
        m.verify_windows_helpers(spec,expected)
        with self.assertRaisesRegex(ValueError,'hash'):
            m.verify_windows_helpers(spec,{'$PLUGINSDIR/System.dll':b'changed'})
        with self.assertRaisesRegex(ValueError,'Unexpected'):
            m.verify_windows_helpers(spec,{**expected,'$PLUGINSDIR/unmapped.dll':b'new'})

    def test_lgpl_prefix_does_not_hide_a_supplier_notice_inside_the_implementation(self):
        m=self.module()
        source=b'/* GNU Lesser General Public License */\nint helper() {\n/* Boost Software License. Permission is hereby granted. */\nreturn 0;\n}\n'
        offset,notice=m.supplier_blocks(source)[0]
        stream=io.BytesIO()
        with tarfile.open(fileobj=stream,mode='w') as a:
            item=tarfile.TarInfo('ffmpeg-test/libavutil/inner.c');item.size=len(source);a.addfile(item,io.BytesIO(source))
        archive=stream.getvalue();spec={'bindings':{'ffmpeg':{'sha256':sha(archive)}},'materials':{}}
        with self.assertRaisesRegex(ValueError,'Unmapped native supplier'):
            m.verify_ffmpeg_coverage(spec,archive,'ffmpeg-test',{'libavutil/inner.c'},lambda _name:notice)
        spec['materials']['inner']={'source_file':'inner.txt','sha256':sha(notice),'origin':{
            'kind':'tar_slice','member':'ffmpeg-test/libavutil/inner.c','source_sha256':sha(source),
            'offset':offset,'length':len(notice),'sha256':sha(notice)}}
        m.verify_ffmpeg_coverage(spec,archive,'ffmpeg-test',{'libavutil/inner.c'},lambda _name:notice)

    def test_windows_inspection_normalizes_host_separators_and_refuses_alias_duplicates(self):
        m = self.module()
        self.assertEqual(m.windows_member_names(b'Path = $PLUGINSDIR\\System.dll\r\n'),
                         {'$PLUGINSDIR/System.dll':'$PLUGINSDIR\\System.dll'})
        with self.assertRaisesRegex(ValueError,'Duplicate'):
            m.windows_member_names(b'Path = $PLUGINSDIR/System.dll\nPath = $PLUGINSDIR\\System.dll\n')

    def test_built_package_selection_requires_one_unambiguous_target_artifact(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            for platform,path in [('android','src-tauri/gen/android/app/build/outputs/apk/arm64/release/app.apk'),
                                  ('windows','target/x86_64-pc-windows-msvc/release/bundle/nsis/app-setup.exe')]:
                with self.subTest(platform=platform):
                    with self.assertRaises(ValueError):m.built_package(root,platform)
                    target=root/path;target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(b'package')
                    self.assertEqual(m.built_package(root,platform),target)
                    extra=target.with_name('other'+target.suffix);extra.write_bytes(b'extra')
                    with self.assertRaises(ValueError):m.built_package(root,platform)
                    extra.unlink()

    def test_real_material_payload_generation_keeps_source_bytes_and_refuses_stale_pins(self):
        m = self.module()
        source=SCRIPT.parents[1]
        spec=json.loads((source/m.MATERIAL).read_text())
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            names={m.MATERIAL,'scripts/audio-deps/sources.json','package-lock.json',
                   'src-tauri/windows/UPSTREAM.json','.github/workflows/reviewed-release.yml'}
            for item in spec['materials'].values():
                names.add(item['source_file'])
                if item['origin']['kind']=='tracked_slice':names.add(item['origin']['source_file'])
            for name in names:
                p=root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes((source/name).read_bytes())
            (root/'THIRD_PARTY_NOTICES.md').write_text('# Existing notices\n')
            command=[sys.executable,str(SCRIPT),'--root',str(root)]
            result=subprocess.run([*command,'--generate'],capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            result=subprocess.run([*command,'--check'],capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            payload=(root/m.PAYLOAD).read_text()
            self.assertIn('This software is based in part on the work of the Independent JPEG Group.',payload)
            self.assertIn('LZMA',payload)
            original=(root/m.PAYLOAD).read_bytes()
            pin=root/'package-lock.json';data=json.loads(pin.read_text());data['packages']['node_modules/@tauri-apps/cli']['version']='99.0.0';pin.write_text(json.dumps(data))
            result=subprocess.run([*command,'--generate'],capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertEqual((root/m.PAYLOAD).read_bytes(),original)


if __name__ == '__main__':
    unittest.main()
