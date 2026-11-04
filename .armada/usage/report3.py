import pickle,os,collections,statistics as st
d=pickle.load(open(os.path.join(os.path.dirname(os.path.abspath(__file__)),'data3.pkl'),'rb'))
seqs=d['seqs']
print('runs',dict(d['runs']));print('run extra',d['run_extra'])
print('book',{k:(v[0],round(v[1])) for k,v in d['book'].items()})
print('skills',d['skills'].most_common(40));print('nskills',len(d['skills']))
print('mcp',d['mcp'].most_common(20));print('agents',d['atypes'].most_common(16));print('toolsearch',d['tsq'].most_common(8))
for k,v in d['first'].items():
    print('\nFIRST',k,'n',len(v),'C median',st.median(x['C'] for x in v),'tools chars',st.median(x['tools'] for x in v),'ntools',st.median(x['ntools'] for x in v),'sys chars',st.median(sum(x['sys']) for x in v),'usr chars',st.median(x['usr'] for x in v))
    keys=set(a for x in v for a in x['att'])
    print({a:round(st.median(x['att'].get(a,0) for x in v)) for a in sorted(keys,key=lambda a:-st.median(x['att'].get(a,0) for x in v))[:14]})
s=d['sys'].get('sys')
if s:
    print('sys parts',[len(x) for x in s])
    t=s[-1]
    import re
    for m in re.finditer(r'\n([A-Z][A-Za-z ]+\.|The Logbook|How [a-z ]+|What this Mac has)[^\n]{0,60}',t):
        print(m.start(),repr(m.group(0)[:70]))
# simulate
P_w={'main':2.0,'subagent':1.25}
def actual(kind,seq):
    return sum((cr*.1+cw*P_w[kind]+i+o*5)*p/1e6 for C,cr,cw,i,o,p,t in seq)
A=sum(actual(k,s) for k,_,s in seqs);print('\nactual',round(A))
def sim(W=None,R=70000,reread=0,cold_compact=False,baseline_cut=0,clear=None):
    tot=0.0
    for kind,_,seq in seqs:
        ttl=3600 if kind=='main' else 300
        wr=P_w[kind]
        cur=None;prevt=None;prevreal=None;offset=0
        for C,cr,cw,i,o,p,t in seq:
            P=p/1e6
            if prevreal is None:
                delta=C;cur=C-baseline_cut;tot+=(cur*wr+o*5)*P if True else 0
                prevreal=C;prevt=t;continue
            if C<0.6*prevreal: # real compaction in the data; resets too
                delta=None
            else: delta=C-prevreal
            gap=t-prevt if t and prevt else 0
            cold=gap>=ttl
            if delta is None:
                # real compaction: keep simulated size if a smaller W already compacted, else follow it
                newc=min(cur,C-baseline_cut) if W else C-baseline_cut
                if W is None:
                    tot+=(cur*.1+o*5)*P+ (newc*wr)*P
                else:
                    tot+=(newc*.1+o*5)*P
                cur=newc
            else:
                newc=cur+max(0,delta)
                didc=False
                if (W and newc>W) or (cold_compact and cold and newc>R+40000):
                    # compaction turn: read old (cold: at 1x), summary out 6k, rewrite R, reread
                    tot+=(cur*(1.0 if cold else .1)+6000*5)*P
                    newc=R+reread;tot+=newc*wr*P+o*5*P+0
                    didc=True
                elif cold:
                    tot+=(newc*wr+o*5)*P
                else:
                    tot+=(cur*.1+max(0,delta)*wr+o*5)*P
                cur=newc
            prevreal=C;prevt=t
    return tot
base=sim()
print('sim base',round(base),'vs actual',round(A))
for W in(300000,250000,200000,150000):
    for rr in(0,20000,40000):
        v=sim(W=W,reread=rr);print('W',W,'reread',rr,round(v),'%.1f%%'%(100*(base-v)/base))
for rr in(0,20000):
    v=sim(cold_compact=True,reread=rr);print('cold-compact reread',rr,round(v),'%.1f%%'%(100*(base-v)/base))
v=sim(W=250000,cold_compact=True,reread=20000);print('250k+cold',round(v),'%.1f%%'%(100*(base-v)/base))
for b in(5000,10000,20000,25000):
    v=sim(baseline_cut=b);print('baseline cut',b,round(v),'%.1f%%'%(100*(base-v)/base))
# context stats
reqs=[(k,C) for k,_,s in seqs for C,*_ in s]
for k in('main','subagent'):
    z=[C for kk,C in reqs if kk==k];print(k,'mean ctx',round(sum(z)/len(z)),'median',st.median(z),'req',len(z))
# post-compaction size
post=[]
for k,_,s in seqs:
    for a,b in zip(s,s[1:]):
        if b[0]<.6*a[0]:post.append((k,a[0],b[0]))
print('compactions',len(post),'pre median',st.median(x[1] for x in post),'post median',st.median(x[2] for x in post))
