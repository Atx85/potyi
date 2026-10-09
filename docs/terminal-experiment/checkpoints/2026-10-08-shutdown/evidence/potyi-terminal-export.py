import difflib,hashlib,json,pathlib,subprocess
base=pathlib.Path('/private/tmp/potyi-term-baseline')
root=pathlib.Path('/private/tmp/potyi-term-integration')
shared=pathlib.Path('/Users/abanko/Desktop/untitled folder/potyi')
out=shared/'docs/terminal-experiment'
metadata=json.loads((out/'files.json').read_text())
files=json.loads(pathlib.Path('/private/tmp/potyi-terminal-milestone-files.json').read_text())
chunks=[]
for name in files:
    source=root/name
    before=base/name
    current=source.read_bytes()
    old=before.read_bytes() if before.exists() else b''
    if old==current:
        continue
    assert b'\0' not in old+current,name
    chunks.append(f'diff --git a/{name} b/{name}\n')
    if not before.exists(): chunks.append('new file mode 100644\n')
    diff=difflib.unified_diff(old.decode().splitlines(keepends=True),current.decode().splitlines(keepends=True),fromfile='a/'+name if before.exists() else '/dev/null',tofile='b/'+name,n=3)
    for line in diff:
        chunks.append(line)
        if not line.endswith('\n'): chunks.append('\n\\ No newline at end of file\n')
patch=out/'integration.patch'
patch.write_text(''.join(chunks))
for name in metadata['unchanged_legacy_files']:
    assert (base/name).read_bytes()==(root/name).read_bytes(),name
metadata['experiment_files']=files
metadata['file_sha256']={name:hashlib.sha256((root/name).read_bytes()).hexdigest() for name in files}
metadata['patch_sha256']=hashlib.sha256(patch.read_bytes()).hexdigest()
metadata['storage']='Two bounded 8 MiB disk rings: browsing and terminal history, each including its on-disk index'
metadata['milestone']='Native browsing, owned prompt, quick file opens and managed command lifecycle'
(out/'files.json').write_text(json.dumps(metadata,indent=2)+'\n')
report=(root/'docs/terminal-experiment.md').read_text().replace('(../tools/qa/terminal-backend/README.md)','(/private/tmp/potyi-term-integration/tools/qa/terminal-backend/README.md)')
(out/'comparison.md').write_text(report)
result=subprocess.run(['git','apply','--check',str(patch)],cwd=base,capture_output=True,text=True)
assert result.returncode==0,result.stderr
print(json.dumps({'files':len(files),'patch_bytes':patch.stat().st_size,'legacy_unchanged':len(metadata['unchanged_legacy_files']),'baseline_apply':'pass'}))
