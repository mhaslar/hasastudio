"""Portable regression checks for the independent readback auditor; no GPU needed."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('audit', Path(__file__).with_name('audit-colour16.py'))
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)
FIXTURE = Path(__file__).resolve().parents[1] / 'docs/testing/phase-1-colour16-macos-aarch64'


class AuditTests(unittest.TestCase):
    def test_actual_rgba16_and_raw_evidence_reconstructs_metrics(self):
        result = AUDIT.audit(FIXTURE)
        self.assertEqual(sum(c['pixels'] for c in result['cases']), 83525)
        self.assertTrue(all(c['max_png16_egress_code_value_error'] <= 2 for c in result['cases']))
        rgba = AUDIT.decode_png((FIXTURE / 'black.png').read_bytes(), 16)
        values = [v[0] for v in AUDIT.struct.iter_unpack('>H', rgba)]
        self.assertTrue(any(v % 257 for v in values), '16-bit output was just expanded from 8-bit')

    def test_forged_linear_summary_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp) / 'evidence'
            shutil.copytree(FIXTURE, directory)
            report = json.loads((directory / 'report.json').read_text())
            report['cases'][0]['max_linear_absolute_error'] = 0
            (directory / 'report.json').write_text(json.dumps(report))
            with self.assertRaisesRegex(ValueError, 'linear maximum cannot be reproduced'):
                AUDIT.audit(directory)

    def test_changed_raw_bits_fail_even_if_their_hash_is_updated(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp) / 'evidence'
            shutil.copytree(FIXTURE, directory)
            path = directory / 'black.rgba16f.le'
            raw = bytearray(path.read_bytes())
            raw[:2] = b'\0\0'  # Erase the discriminating white-over-black red sample.
            path.write_bytes(raw)
            report = json.loads((directory / 'report.json').read_text())
            report['cases'][0]['linear_readback']['sha256'] = AUDIT.digest(raw)
            (directory / 'report.json').write_text(json.dumps(report))
            with self.assertRaisesRegex(ValueError, 'linear maximum cannot be reproduced'):
                AUDIT.audit(directory)

    def test_png_crc_and_depth_are_checked(self):
        original = (FIXTURE / 'black.png').read_bytes()
        broken = bytearray(original)
        broken[29] ^= 1
        with self.assertRaisesRegex(ValueError, 'PNG CRC mismatch'):
            AUDIT.decode_png(broken, 16)
        with self.assertRaisesRegex(ValueError, 'dimensions/depth/format'):
            AUDIT.decode_png(original, 8)


if __name__ == '__main__':
    unittest.main()
