"""Focused license staging and archive-content checks; no app builds or execution."""
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest
import io
import tarfile
from unittest.mock import patch

SCRIPT=Path(__file__).resolve().parents[1]/'appimage-licenses.py'


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


class AppImageLicenses(unittest.TestCase):
    def module(self):
        self.assertTrue(SCRIPT.is_file(),'Missing AppImage shared-license staging/checker')
        spec=importlib.util.spec_from_file_location('appimage_licenses',SCRIPT)
        m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
        return m

    def fixture(self,root,m):
        for name in ['SOURCES.json','MPL-1.1','MPL-2.0']:
            src=SCRIPT.parents[1]/'src/licenses/appimage-common'/name
            dst=root/'src/licenses/appimage-common'/name;dst.parent.mkdir(parents=True,exist_ok=True);dst.write_bytes(src.read_bytes())
        host=root/'host';host.mkdir()
        source=json.loads((root/m.SOURCES).read_text())
        for name in source['required_names']:
            if not name.startswith('MPL-'):(host/name).write_bytes(('Exact host '+name+' license\r\n').encode())
        return host

    def test_stage_materializes_host_aliases_and_supplies_verified_mozilla_fallbacks(self):
        m=self.module()
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);host=self.fixture(root,m)
            (host/'GPL').unlink();(host/'GPL').symlink_to('GPL-3')
            manifest=m.stage(root,host)
            out=root/m.OUTPUT
            self.assertEqual((out/'GPL').read_bytes(),(host/'GPL-3').read_bytes())
            self.assertFalse((out/'GPL').is_symlink())
            self.assertEqual((out/'MPL-1.1').read_bytes(),(root/'src/licenses/appimage-common/MPL-1.1').read_bytes())
            self.assertEqual(m.stage(root,host),manifest)
            (host/'GPL-3').write_bytes(b'Updated host bytes\n')
            m.stage(root,host)
            self.assertEqual((out/'GPL-3').read_bytes(),b'Updated host bytes\n')

    def test_stage_refuses_outside_changes_unknown_files_and_escaping_host_links(self):
        m=self.module()
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);host=self.fixture(root,m);m.stage(root,host);out=root/m.OUTPUT
            old=(out/'GPL-2').read_bytes();(out/'GPL-2').write_bytes(b'outside edit')
            with self.assertRaises(ValueError):m.stage(root,host)
            self.assertEqual((out/'GPL-2').read_bytes(),b'outside edit')
            (out/'GPL-2').write_bytes(old);(out/'unknown').write_bytes(b'preserve me')
            with self.assertRaises(ValueError):m.stage(root,host)
            self.assertEqual((out/'unknown').read_bytes(),b'preserve me')
            (out/'unknown').unlink();(host/'GPL').unlink();(host/'GPL').symlink_to(root/'outside')
            (root/'outside').write_bytes(b'outside source')
            with self.assertRaises(ValueError):m.stage(root,host)

    def test_finished_image_requires_exact_staged_terms_for_every_copyright_reference(self):
        m=self.module()
        manifest={'schema':1,'owner':'taggroove-appimage-licenses','files':{
            'GPL-2':{'sha256':sha(b'Full GPL2'), 'size':len(b'Full GPL2')}}}
        paths={'usr/share/doc/library/copyright':{'Folder':'-','Mode':'-rw-r--r--'}}
        data={'usr/share/doc/library/copyright':b'Full text: /usr/share/common-licenses/GPL-2.\n'}
        with self.assertRaises(ValueError):m.verify_members(paths,manifest,data.__getitem__)
        paths['usr/share/common-licenses/GPL-2']={'Folder':'-','Mode':'-rw-r--r--'}
        data['usr/share/common-licenses/GPL-2']=b'Full GPL2'
        m.verify_members(paths,manifest,data.__getitem__)
        data['usr/share/doc/library/copyright']+=b'/usr/share/common-licenses/Unknown\n'
        with self.assertRaisesRegex(ValueError,'reference'):m.verify_members(paths,manifest,data.__getitem__)
        data['usr/share/doc/library/copyright']=b'/usr/share/common-licenses/GPL-2\n';data['usr/share/common-licenses/GPL-2']=b'wrong bytes'
        with self.assertRaisesRegex(ValueError,'hash'):m.verify_members(paths,manifest,data.__getitem__)

    def test_stage_preserves_files_created_after_ownership_scan(self):
        for relative in ['target/appimage-common-licenses/GPL-2','target/appimage-license-stage.json']:
            with self.subTest(relative=relative),tempfile.TemporaryDirectory() as folder:
                m=self.module();root=Path(folder);host=self.fixture(root,m)
                publish=m.core.publish_outputs
                outside=root/relative
                def inject(*args,**kwargs):
                    outside.parent.mkdir(parents=True,exist_ok=True)
                    outside.write_bytes(b'outside arrival')
                    return publish(*args,**kwargs)
                with patch.object(m.core,'publish_outputs',side_effect=inject):
                    with self.assertRaisesRegex(ValueError,'appeared'):m.stage(root,host)
                self.assertEqual(outside.read_bytes(),b'outside arrival')
                self.assertFalse((root/m.OUTPUT/'MPL-2.0').exists())

    def test_inspector_cache_requires_pinned_bytes_and_repairs_only_known_execute_mode(self):
        m=self.module()
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);self.fixture(root,m)
            metadata=json.loads((root/m.SOURCES).read_text());record=next(x for x in metadata['inspector']['archives'] if x['arch']=='x86_64')
            binary=b'fixture inspection executable; never run'
            stream=io.BytesIO()
            with tarfile.open(fileobj=stream,mode='w:xz') as archive:
                entry=tarfile.TarInfo('7zz');entry.size=len(binary);archive.addfile(entry,io.BytesIO(binary))
            raw=stream.getvalue();record['sha256']=sha(raw);record['binary_sha256']=sha(binary)
            (root/m.SOURCES).write_text(json.dumps(metadata))
            with patch.object(m.platform,'system',return_value='Linux'),patch.object(m.platform,'machine',return_value='x86_64'):
                with patch.object(m.urllib.request,'urlopen',return_value=io.BytesIO(raw)):
                    tool=m.prepare_inspector(root)
                self.assertEqual(tool.read_bytes(),binary)
                tool.chmod(0o644);m.prepare_inspector(root)
                self.assertTrue(tool.stat().st_mode&0o111)
                tool.write_bytes(b'outside changed executable')
                with self.assertRaises(ValueError):m.prepare_inspector(root)
                self.assertEqual(tool.read_bytes(),b'outside changed executable')

    def test_inspector_preserves_file_created_during_download(self):
        m=self.module()
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);self.fixture(root,m)
            metadata=json.loads((root/m.SOURCES).read_text())
            record=next(x for x in metadata['inspector']['archives'] if x['arch']=='x86_64')
            binary=b'fixture inspection executable; never run';stream=io.BytesIO()
            with tarfile.open(fileobj=stream,mode='w:xz') as archive:
                entry=tarfile.TarInfo('7zz');entry.size=len(binary);archive.addfile(entry,io.BytesIO(binary))
            raw=stream.getvalue();record['sha256']=sha(raw);record['binary_sha256']=sha(binary)
            (root/m.SOURCES).write_text(json.dumps(metadata))
            with patch.object(m.platform,'system',return_value='Linux'),patch.object(m.platform,'machine',return_value='x86_64'):
                _record,tool=m.inspector_record(root)
                def download(*args,**kwargs):
                    tool.parent.mkdir(parents=True,exist_ok=True);tool.write_bytes(b'outside arrival')
                    return io.BytesIO(raw)
                with patch.object(m.urllib.request,'urlopen',side_effect=download):
                    with self.assertRaisesRegex(ValueError,'appeared'):m.prepare_inspector(root)
                self.assertEqual(tool.read_bytes(),b'outside arrival')


if __name__=='__main__':unittest.main()
