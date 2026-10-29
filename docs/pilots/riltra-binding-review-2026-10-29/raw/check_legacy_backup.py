import os,json,subprocess,pathlib,hashlib
root=pathlib.Path("target/review-fixes").resolve()
repo=root/"legacy2-repo"; repo.mkdir()
(repo/"a.rs").write_text("pub fn f() {}\n",encoding="utf-8")
new=pathlib.Path("target/debug/riltra.exe").resolve()
old=root/"baseline-build/debug/riltra.exe"
log=[]
def run(exe,home,args,data=None):
 env=os.environ.copy();env["RILTRA_HOME"]=str(home)
 p=subprocess.run([str(exe),*args],input=None if data is None else json.dumps(data),capture_output=True,text=True,encoding="utf-8",env=env)
 log.append({"binary":"baseline" if exe==old else "current","command":args,"input":data,"exit":p.returncode,"stdout":p.stdout,"stderr":p.stderr})
 if p.returncode: raise RuntimeError(str(log[-1]))
 return p.stdout
home=root/"legacy2-current"
run(new,home,["init","--repo",str(repo),"--name","legacy"])
for id in ("d1","d2"):
 run(new,home,["knowledge","create","--repo","legacy","--input","-"],{"id":id,"kind":"decision","title":id,"body":"fixture","status":"active","author":"fixture","evidence":[]})
for stage in (1,2):
 if stage==2:
  run(new,home,["knowledge","link","--repo","legacy","--input","-"],{"relation_id":"r1","src_id":"d1","src_revision":1,"dst_id":"d2","dst_revision":1,"kind":"relates_to","author":"fixture","rationale":"fixture","confirm_state":"confirmed"})
 base=root/("legacy2-backup-v"+str(stage))
 run(new,home,["knowledge","backup","--repo","legacy","--out",str(base)])
 manifest=json.loads(pathlib.Path(str(base)+".manifest.json").read_text(encoding="utf-8-sig"))
 assert manifest["schema_version"]==stage and manifest["version"]==1 and "code_bindings" not in manifest,manifest
 oldhome=root/("legacy2-old-home-"+str(stage))
 run(old,oldhome,["init","--repo",str(repo),"--name","legacy"])
 run(old,oldhome,["knowledge","restore","--repo","legacy","--file",str(base)])
 show=json.loads(run(old,oldhome,["knowledge","show","--repo","legacy","--id","d1"]))
 assert show["result"]["id"]=="d1",show
print(json.dumps({"schema_v1_and_v2_restored_by_baseline":True,"baseline_sha256":hashlib.sha256(old.read_bytes()).hexdigest(),"commands":len(log)}))
(root/"legacy-backup-evidence.json").write_text(json.dumps(log,indent=2),encoding="utf-8")
