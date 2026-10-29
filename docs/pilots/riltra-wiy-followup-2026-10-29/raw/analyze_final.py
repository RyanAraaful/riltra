import json, os, pathlib, subprocess, time, csv, collections
p=pathlib.Path("target/wiy-followup")
env=os.environ.copy(); env["RILTRA_HOME"]=str((p/"home-final").resolve())
proc=subprocess.Popen(["target/release/riltra.exe","mcp"],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding="utf-8",env=env)
def send(x): proc.stdin.write(json.dumps(x,separators=(",",":"))+"\n"); proc.stdin.flush()
send({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"wiy-followup","version":"1"}}})
time.sleep(1)
send({"jsonrpc":"2.0","method":"notifications/initialized"})
args={"repo":"riltra-e2-confined","path":"crates/cli/src/semantic.rs","line":276,"col":8,"limit":1000}
send({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"riltra_code_callers","arguments":args}})
time.sleep(1)
send({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"riltra_code_impact","arguments":args}})
time.sleep(5); proc.stdin.close()
out=proc.stdout.read(); err=proc.stderr.read(); code=proc.wait(timeout=30)
(p/"mcp-final.stdout").write_text(out,encoding="utf-8"); (p/"mcp-final.stderr").write_text(err,encoding="utf-8"); (p/"mcp-final.exit").write_text(str(code),encoding="utf-8")
responses={v["id"]:v["result"]["structuredContent"] for v in map(json.loads,out.splitlines()) if v.get("id") in (2,3)}
calls=json.loads((p/"callers-final.stdout").read_text(encoding="utf-8-sig")); impact=json.loads((p/"impact-final.stdout").read_text(encoding="utf-8-sig"))
with open("docs/pilots/riltra-what-impact-why-implementation-2026-10-29/raw/15-callsite-accounting.csv",encoding="utf-8-sig",newline="") as f: oracle=list(csv.DictReader(f))
rows=[]; matched=set()
for row in oracle:
 lo,hi=int(row["byte_start"]),int(row["byte_end"]); hits=[]
 for i,e in enumerate(calls["edges"]):
  a,b=e["caller_span"]
  if e["caller_path"]==row["path"] and (e["kind"] in ("CALLS","POTENTIAL_CALL") and (a,b)==(lo,hi) or e["kind"]=="REFERENCES" and lo<=a<b<=hi):
   hits.append({"kind":e["kind"],"span":[a,b]}); matched.add(i)
 rows.append({"path":row["path"],"line":int(row["oracle_line"]),"hits":hits})
result={"oracle_rows":len(rows),"rows":rows,"unmatched_edges":len(calls["edges"])-len(matched),"kinds":dict(collections.Counter(e["kind"] for e in calls["edges"])),"mcp_callers_equal":calls==responses[2],"cli_impact_verdict":impact["verdict"],"mcp_impact_verdict":responses[3]["verdict"],"mcp_impact_equal":impact==responses[3],"mcp_exit":code,"mcp_stderr_empty":not err}
(p/"analysis-final.json").write_text(json.dumps(result,indent=2),encoding="utf-8")
print(json.dumps({k:v for k,v in result.items() if k!="rows"}))
if code or err or len(rows)!=13 or any(len(row["hits"])!=1 for row in rows) or result["unmatched_edges"] or not result["mcp_callers_equal"]: raise SystemExit(1)
