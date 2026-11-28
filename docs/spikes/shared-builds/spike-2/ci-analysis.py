import json,collections,re,sys,os
os.chdir(os.path.dirname(os.path.abspath(__file__)))
mg=json.load(open('ci-merge-group-runs.json'))
pr=json.load(open('ci-pull-request-runs.json'))
pl=json.load(open('ci-pr-list-100.json'))
nums={p['number'] for p in pl}
byhead=collections.defaultdict(list)
for r in pr: byhead[r['headSha']].append(r)
red_last=red_any=norun=0
for p in pl:
    rs=sorted(byhead.get(p['headRefOid'],[]),key=lambda r:r['createdAt'])
    rs=[r for r in rs if r['conclusion'] in('success','failure')]
    if not rs: norun+=1; continue
    if rs[-1]['conclusion']=='failure': red_last+=1
    if any(r['conclusion']=='failure' for r in rs): red_any+=1
print("PR final heads:",len(pl),"with a finished pull_request run",len(pl)-norun,"last run red",red_last,"any red",red_any)
att=collections.defaultdict(list)
for r in mg:
    m=re.search(r'pr-(\d+)-',r['headBranch'])
    n=int(m.group(1)) if m else None
    att[n].append(r)
c=collections.Counter(r['conclusion'] for r in mg if r['conclusion'])
print("merge_group runs",len(mg),dict(c))
s,f,x=c['success'],c['failure'],c['cancelled']
print("p (failure/(success+failure)) = %d/%d = %.3f"%(f,s+f,f/(s+f)))
print("p incl cancelled as non-green = %d/%d = %.3f"%(f+x,s+f+x,(f+x)/(s+f+x)))
sub=[r for n in nums for r in att.get(n,[])]
c2=collections.Counter(r['conclusion'] for r in sub)
print("entries of the 100 merged PRs:",len(sub),dict(c2))
s2,f2=c2['success'],c2['failure']; print("p100 = %d/%d = %.3f"%(f2,s2+f2,f2/(s2+f2)))
fp=[n for n,rs in att.items() if any(r['conclusion']=='failure' for r in rs)]
print("PRs with >=1 failed queue attempt:",len(fp),"of",len(att),"; of the 100 merged:",len([n for n in fp if n in nums]))
print("merged PRs with no queue run seen:",len([n for n in nums if n not in att]))
for N in (1,2,4,5,8):
    for name,p in (("p_all",f/(s+f)),("p_100",f2/(s2+f2))):
        print("N=%d %s batch red = %.3f"%(N,name,1-(1-p)**N))
