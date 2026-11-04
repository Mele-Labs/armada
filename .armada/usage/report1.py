import pickle,os,collections,statistics as st
d=pickle.load(open(os.path.join(os.path.dirname(__file__),'data.pkl'),'rb'))
tot=d['tot'];B=d['bucket']
kinds=['main','subagent','bg-temp','bg-tmp']
grand=sum(tot[k].get('c_read',0)+tot[k].get('c_write',0)+tot[k].get('c_out',0) for k in kinds if k in tot)
print('GRAND $eq',round(grand,1))
for k in kinds:
    if k not in tot: continue
    T=tot[k];c=T['c_read']+T['c_write']+T['c_out']
    print(k,'share %.1f%%'%(100*c/grand),'cost %.0f'%c,'req %d'%T['requests'],'read %.0f%% write %.0f%% out %.0f%%'%(100*T['c_read']/c,100*T['c_write']/c,100*T['c_out']/c),'w1h %.0fM w5m %.0fM cr %.0fM out %.1fM think %.1fM'%(T['w1']/1e6,T['w5']/1e6,T['cache_read']/1e6,T['output']/1e6,T['thinking']/1e6))
print({k:(round(v['cost']),int(v['requests'])) for k,v in tot.items() if k.startswith('model:')})
print('\nBUCKETS all kinds')
agg=collections.defaultdict(float)
for k in kinds:
    for c,v in B.get(k,{}).items(): agg[c]+=v
for c,v in sorted(agg.items(),key=lambda x:-x[1])[:32]: print('%6.2f%%  %8.0f  %s'%(100*v/grand,v,c))
for k in ['main','subagent']:
    print('\nBUCKETS',k); 
    for c,v in sorted(B[k].items(),key=lambda x:-x[1])[:16]: print('%6.2f%%  %s'%(100*v/grand,c))
print('\nTRIGGER (what the request follows) cost, n, avg')
for k in ['main','subagent']:
    print(k)
    for c,v in sorted(d['trigger'][k].items(),key=lambda x:-x[1])[:16]:
        n=d['trigger_n'][k][c]; print('%6.2f%% n=%6d avg=%.3f %s'%(100*v/grand,n,v/n,c))
print('\nCTX bands')
for k in ['main','subagent','bg-temp']:
    print(k,{b:('%.1f%%'%(100*v/grand),int(d['ctxband'][k+':n'].get(b,0))) for b,v in sorted(d['ctxband'][k].items())})
th=d['threads']
print('\nTHREADS')
for k in kinds:
    t=[x for x in th if x['kind']==k and x['req']>0]
    if not t: continue
    print(k,'n',len(t),'median req',st.median(x['req'] for x in t),'median first ctx',st.median(x['first'] or 0 for x in t),'median max',st.median(x['max'] for x in t),'p90 max',sorted(x['max'] for x in t)[int(.9*len(t))],'cold',sum(x['cold'] for x in t),'coldcost %.0f'%sum(x['coldcost'] for x in t),'shapechg',sum(x['shapechg'] for x in t))
t=sorted([x for x in th if x['kind'] in('main','subagent')],key=lambda x:-x['cost'])
c=[x['cost'] for x in t]; s=sum(c)
print('top1%% threads %.0f%% top10%% %.0f%%'%(100*sum(c[:len(c)//100])/s,100*sum(c[:len(c)//10])/s))
at=collections.defaultdict(lambda:[0,0.0,0])
for x in th:
    if x['kind']=='subagent': a=at[x['agent'] or '?'];a[0]+=1;a[1]+=x['cost'];a[2]+=x['req']
for k,v in sorted(at.items(),key=lambda x:-x[1][1])[:12]: print('agent',k,v[0],'%.1f%%'%(100*v[1]/grand),'req',v[2])
sm=[x for x in d['shapes'].values()]
sm.sort(key=lambda x:-x['n'])
print('\nSHAPES n=%d'%len(sm))
for s in sm[:6]: print(s['n'],'sys',s['sys'],'tools',s['tools'],'ntools',len(s['names']),s['kinds'],s['parts'])
