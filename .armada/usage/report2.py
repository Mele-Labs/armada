import pickle,os,collections,statistics as st
d=pickle.load(open(os.path.join(os.path.dirname(os.path.abspath(__file__)),'data2.pkl'),'rb'))
R=d['R'];reads=d['reads'];th=d['threads']
T=R['totals']
grand=sum(T[k]['cost'] for k in T)
print('grand (excl temp bg)',round(grand))
for k in T: print(k,{a:round(b/1e6,1) if a in('w1','w5','cr','outt','think','in') else round(b) for a,b in T[k].items()})
B=collections.defaultdict(float)
for k in R['bucket']:
    for c,v in R['bucket'][k].items():
        key=c.replace(' (write)','')
        B[key]+=v
print('\nALL BUCKETS (carried+write merged)')
for c,v in sorted(B.items(),key=lambda x:-x[1])[:40]: print('%6.2f%% %7.0f %s'%(100*v/grand,v,c))
for k in('main','subagent'):
    print('\n',k); 
    b=collections.defaultdict(float)
    for c,v in R['bucket'][k].items(): b[c.replace(' (write)','')]+=v
    for c,v in sorted(b.items(),key=lambda x:-x[1])[:14]: print('%6.2f%% %s'%(100*v/grand,c))
print('\nMISS: kind|why n Mtok cost')
for k,v in sorted(R['miss'].items(),key=lambda x:-x[1][2]): print(k,v[0],round(v[1]/1e6,1),round(v[2]),'%.2f%%'%(100*v[2]/grand),{a:round(b) for a,b in R['gaps'].get(k,{}).items()})
print('\nAGE of carried results')
for k in R['age']:
    l=R['age'][k].get('liveresults',1); print(k,{a:'%.0f%%'%(100*b/l) for a,b in R['age'][k].items() if a!='liveresults'},'total$',round(l),'%.1f%% of all'%(100*l/grand))
print('\nCALLS')
for k in('main','subagent'):
    print(k,sorted(((n,v[0]) for n,v in R['calls'][k].items()),key=lambda x:-x[1])[:18])
print('gitnexus calls',{k:{n:v[0] for n,v in R['calls'][k].items() if 'gitnexus' in n} for k in('main','subagent')})
print('tools per request',R['tools_per'])
print('guard',R['guard'])
print('hook add',sorted(((k,v[0],round(v[1]/1e6,2)) for k,v in R['hookadd'].items()),key=lambda x:-x[2])[:8])
print('att',sorted(((k,v[0],round(v[1]/1e6,2)) for k,v in R['att'].items()),key=lambda x:-x[2])[:14])
print('\nTRIG')
for k in('main','subagent'):
    print(k,[(c,int(R['trign'][k][c]),'%.1f%%'%(100*v/grand)) for c,v in sorted(R['trig'][k].items(),key=lambda x:-x[1])[:12]])
# reads
print('\nREADS n',len(reads))
via=collections.defaultdict(lambda:[0,0.0,0.0])
for r in reads:
    x=via[(r['kind'],'Read' if r['via']=='Read' else 'Bash-read')];x[0]+=1;x[1]+=r['tok'];x[2]+=r['carry']
for k,v in via.items(): print(k,v[0],'Mtok %.1f'%(v[1]/1e6),'carry$ %.0f %.2f%%'%(v[2],100*v[2]/grand))
rr=[r for r in reads if r['via']=='Read' and not r.get('away')]
print('Read: whole',sum(1 for r in rr if r['whole']),'ranged',sum(1 for r in rr if not r['whole']),'whole Mtok %.1f ranged Mtok %.1f'%(sum(r['tok'] for r in rr if r['whole'])/1e6,sum(r['tok'] for r in rr if not r['whole'])/1e6))
print('turned away',sum(1 for r in reads if r.get('away')))
sz=sorted(r['tok'] for r in rr); print('Read size p50 %d p90 %d p99 %d max %d'%(sz[len(sz)//2],sz[int(.9*len(sz))],sz[int(.99*len(sz))],sz[-1]))
big=sum(r['tok'] for r in rr if r['tok']>2500);print('Reads >2.5k tok share of tokens %.0f%% n=%d'%(100*big/sum(sz),sum(1 for r in rr if r['tok']>2500)))
allr=[r for r in reads if r['p'] and not r.get('away')]
# repeat within thread
seen=set();rep=0;rept=0;repc=0;tot=0;totc=0
for r in allr:
    key=(r['thr'],r['p'],r.get('off'),r.get('lim'))
    tot+=r['tok'];totc+=r['carry']
    if key in seen and r['via']=='Read': rep+=1;rept+=r['tok'];repc+=r['carry']
    seen.add(key)
print('same-range re-read in thread n=%d tok=%.1fM (%.0f%%) carry=%.0f'%(rep,rept/1e6,100*rept/tot,repc))
# same file reread in thread any range
seen=set();rep=0;rept=0;repc=0
for r in allr:
    key=(r['thr'],r['p'])
    if key in seen: rep+=1;rept+=r['tok'];repc+=r['carry']
    seen.add(key)
print('same-file again in thread n=%d tok=%.1fM (%.0f%%) carry=%.0f'%(rep,rept/1e6,100*rept/tot,repc))
# session overlap: file already read by another thread of the same session
by=collections.defaultdict(set);ovl=0;ovc=0;ovn=0;subt=0;subc=0
for r in sorted(allr,key=lambda x:(x['kind']!='main')):
    k=(r['sess'],r['p'])
    if r['kind']=='subagent':
        subt+=r['tok'];subc+=r['carry']
        if by[k]-{r['thr']}: ovl+=r['tok'];ovc+=r['carry'];ovn+=1
    by[k].add(r['thr'])
print('subagent reads of a file another thread of the session read: n=%d tok=%.1fM of %.1fM (%.0f%%) carry=%.0f'%(ovn,ovl/1e6,subt/1e6,100*ovl/max(1,subt),ovc))
# across sessions
fs=collections.defaultdict(lambda:[set(),0.0,0.0,0])
for r in allr:
    x=fs[r['p']];x[0].add(r['sess']);x[1]+=r['tok'];x[2]+=r['carry'];x[3]+=1
tok=sum(v[1] for v in fs.values());carr=sum(v[2] for v in fs.values())
ord_=sorted(fs.items(),key=lambda x:-x[1][1])
print('distinct files',len(fs),'sessions',len(set(r['sess'] for r in allr)))
c=0
for i,(p,v) in enumerate(ord_[:30]):
    print(i+1,p,'sessions',len(v[0]),'reads',v[3],'Ktok %.0f'%(v[1]/1e3),'carry %.0f'%v[2])
for n in(10,50,100,500):
    print('top',n,'files %.0f%% of read tokens'%(100*sum(v[1] for p,v in ord_[:n])/tok))
multi=sum(v[1] for v in fs.values() if len(v[0])>=5);print('files read in >=5 sessions: tokens share %.0f%%'%(100*multi/tok),sum(1 for v in fs.values() if len(v[0])>=5))
ed=sum(r['tok'] for r in allr if r.get('edited'));print('reads of files the thread later edited %.0f%%'%(100*ed/tok))
print('doc reads share %.0f%%'%(100*sum(v[1] for p,v in fs.items() if p.endswith('.md'))/tok),'rs %.0f%%'%(100*sum(v[1] for p,v in fs.items() if p.endswith('.rs'))/tok),'ts %.0f%%'%(100*sum(v[1] for p,v in fs.items() if p.endswith(('.ts','.tsx')))/tok))
# threads
for k in('main','subagent'):
    t=[x for x in th if x['kind']==k and x['req']>0]
    print(k,'threads',len(t),'compactions',sum(x['comp'] for x in t),'with compaction',sum(1 for x in t if x['comp']),'no edit at all',sum(1 for x in t if x['first_edit'] is None),'median first edit req',st.median([x['first_edit'] for x in t if x['first_edit'] is not None] or [0]),'before edit$ %.0f %.1f%%'%(sum(x['before_edit'] for x in t if x['first_edit'] is not None),100*sum(x['before_edit'] for x in t if x['first_edit'] is not None)/grand),'no-edit$ %.0f %.1f%%'%(sum(x['before_edit'] for x in t if x['first_edit'] is None),100*sum(x['before_edit'] for x in t if x['first_edit'] is None)/grand))
