import json
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path
import processor
class ProtocolTests(unittest.TestCase):
    def test_passthrough_preserves_source(self):
        with tempfile.TemporaryDirectory() as folder:
            source=Path(folder)/'source.wav'; output=Path(folder)/'result.wav'
            source.write_bytes(b'RIFF'+b'\0'*80)
            result=processor.process({'config':{'engine':'passthrough'},'sourcePath':str(source),'outputPath':str(output)})
            self.assertEqual(source.read_bytes(),output.read_bytes()); self.assertEqual(result['backend'],'cpu')
    def test_missing_models_not_ready(self):
        result=processor.status({'engine':'seed_vc','modelDir':'/missing/homer-models'})
        self.assertFalse(result['ready']); self.assertIn('inference.py',result['missingFiles'])
    def test_rejects_invalid_protocol_and_oversized_frame(self):
        for data in [b'{"protocol":2}\n',b'x'*(1024*1024+2)]:
            child=subprocess.run([sys.executable,str(Path(processor.__file__))],input=data,capture_output=True)
            self.assertEqual(child.returncode,1); self.assertEqual(json.loads(child.stdout)['type'],'error')
    def test_invalid_parameter_rejected(self):
        for bad in [float('nan'),1000,'30',True]:
            with self.assertRaises(ValueError): processor.number({'steps':bad},'steps',30,1,200,True)
    def test_empty_model_files_and_cache_are_not_ready(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            for name in processor.model_files('seed_vc', root):
                target = root/name; target.parent.mkdir(parents=True, exist_ok=True); target.touch()
            (root/'checkpoints/hf_cache').mkdir()
            with patch('processor.importlib.util.find_spec', return_value=None):
                result = processor.status({'engine':'seed_vc','modelDir':folder})
            self.assertFalse(result['ready']); self.assertIn('inference.py', result['missingFiles'])
            self.assertTrue(any('hf_cache' in name for name in result['missingFiles']))
    def test_same_path_and_non_object_parameters_are_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            source=Path(folder)/'source.wav';source.write_bytes(b'RIFF'+b'0'*80)
            with self.assertRaisesRegex(ValueError, 'new file'):
                processor.process({'config':{'engine':'passthrough'},'sourcePath':str(source),'outputPath':str(source)})
            with self.assertRaisesRegex(ValueError, 'object'):
                processor.process({'config':{'engine':'passthrough'},'sourcePath':str(source),'outputPath':str(Path(folder)/'out.wav'),'params':[]})
    def test_unknown_action_returns_one_error_frame(self):
        child=subprocess.run([sys.executable,str(Path(processor.__file__))],input=b'{"protocol":1,"action":"wrong","config":{}}\n',capture_output=True)
        self.assertEqual(child.returncode,1);self.assertEqual(len(child.stdout.splitlines()),1)
        self.assertEqual(json.loads(child.stdout)['type'],'error')
if __name__=='__main__': unittest.main()

