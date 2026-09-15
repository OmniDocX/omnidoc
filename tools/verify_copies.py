"""Verify ordinary source copies against the pinned public snapshot manifest."""
from pathlib import Path
import hashlib,json,subprocess
root=Path(__file__).resolve().parents[1]
manifest=json.loads((root/'sources.lock.json').read_text(encoding='utf-8'))
assert {p['name'] for p in manifest['projects']}=={'UniPPT','unicell','vecmeta'}
assert not (root/'projects/PolyglotPDF').exists()
index=subprocess.check_output(['git','-C',str(root),'ls-files','-z']).decode().split('\0')
for project in manifest['projects']:
    prefix=project['directory']+'/'
    expected=project['files']
    actual={name[len(prefix):] for name in index if name.startswith(prefix)}
    assert actual==set(expected), f"{project['name']}: missing or extra indexed files"
    for name,expected_hash in expected.items():
        assert hashlib.sha256((root/project['directory']/name).read_bytes()).hexdigest()==expected_hash, f"Changed copy: {project['name']}/{name}"
    print(f"{project['name']}: {len(expected)} files match {project['commit']}")
print('All copied sources verified; PolyglotPDF excluded.')
