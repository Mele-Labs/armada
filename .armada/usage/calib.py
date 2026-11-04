import json,os,pickle,statistics as st,random,collections,mmap,re
base=os.path.dirname(os.path.abspath(__file__))
d=pickle.load(open(base+'/data3.pkl','rb'));seqs=d['seqs']
# 5m TTL on main instead of 1h
cur=0;alt=0;n=0;mis=0
for k,_,s in seqs:
    if k!='main':continue
    pC=None;pt=None
    for C,cr,cw,i,o,p,t in s:
        P=p/1e6
        cur+=(cr*.1+cw*2+i+o*5)*P
        gap=(t-pt) if t and pt else 0
        if pC is not None and 300<=gap<3600:
            alt+=(C*1.25+i+o*5)*P;n+=1;mis+=C
        else:
            alt+=(cr*.1+cw*1.25+i+o*5)*P
        pC=C;pt=t
print('main cost 1h',round(cur),'if 5m',round(alt),'extra cold wakes',n,'avg C',round(mis/max(n,1)))
# growth after compaction vs ordinary
post=[];norm=[]
for k,_,s in seqs:
    cs=[x[0] for x in s]
    for i in range(1,len(cs)-21):
        if cs[i]<.6*cs[i-1]: post.append(cs[i+20]-cs[i])
    for i in range(0,len(cs)-21,21):
        if cs[i]>100000 and min(cs[i:i+21])>.6*cs[i]: norm.append(cs[i+20]-cs[i])
print('20-step growth after compaction median',st.median(post),'mean',round(sum(post)/len(post)),'n',len(post),'| ordinary median',st.median(norm),'mean',round(sum(norm)/len(norm)))
# calibration
ROOT=os.path.expanduser('~/.claude/projects')
paths=[p for _,p,_s in [(a,b,c) for a,b,c in seqs]];random.seed(3);random.shuffle(paths)
pairs=collections.defaultdict(list);think=[];memreads=0
for p in paths[:220]:
    seen={};reqs=[];pend={}
    for l in open(p,errors='replace'):
        try:o=json.loads(l)
        except Exception:continue
        t=o.get('type')
        if t=='assistant':
            m=o['message'];mid=m.get('id');u=m.get('usage') or {}
            if mid not in seen:
                e=dict(u=u,tools=[],vis=0,res=[],other=0);seen[mid]=e;reqs.append(e)
            e=seen[mid];e['u']=u or e['u']
            for b in m.get('content') or []:
                if isinstance(b,dict) and b.get('type')=='text':e['vis']+=len(b.get('text',''))
                if isinstance(b,dict) and b.get('type')=='tool_use':
                    e['tools'].append(b.get('name'));e['vis']+=len(json.dumps(b.get('input') or {}));pend[b['id']]=b.get('name')
        elif t=='user' and reqs:
            c=o['message'].get('content')
            if isinstance(c,list):
                for b in c:
                    if isinstance(b,dict) and b.get('type')=='tool_result':
                        cc=b.get('content');n=len(cc) if isinstance(cc,str) else sum(len(x.get('text','')) for x in cc or [] if isinstance(x,dict) and x.get('type')=='text')
                        reqs[-1]['res'].append((pend.get(b.get('tool_use_id')),n))
                    else: reqs[-1]['other']+=1000
            else: reqs[-1]['other']+=len(c or '')
        elif t=='attachment' and reqs and o.get('rendered'):
            reqs[-1]['other']+=len(json.dumps(o['rendered']))
    for a,b in zip(reqs,reqs[1:]):
        ua=a['u'] or {};ub=b['u'] or {}
        Ca=sum(ua.get(x,0) or 0 for x in('input_tokens','cache_read_input_tokens','cache_creation_input_tokens'))
        Cb=sum(ub.get(x,0) or 0 for x in('input_tokens','cache_read_input_tokens','cache_creation_input_tokens'))
        th=(ua.get('output_tokens_details') or {}).get('thinking_tokens',0) or 0
        if Cb<Ca*.8:continue
        if len(a['res'])==1 and a['other']==0 and a['res'][0][1]>=6000 and a['vis']<800 and th<100:
            pairs[a['res'][0][0]].append((Cb-Ca)/a['res'][0][1])
        if len(a['res'])==1 and a['other']==0 and a['res'][0][1]<400 and a['vis']<400 and th>=1500:
            think.append((Cb-Ca)/th)
for k,v in pairs.items():
    if len(v)>=15:print(k,'n',len(v),'tok/char median %.3f chars/tok %.2f'%(st.median(v),1/st.median(v)))
print('thinking carried: dC per thinking tok median %.2f n=%d'%(st.median(think) if think else -1,len(think)))
# tools schema
for l in open(os.path.expanduser('~/.claude/projects/-Users-nickmele-Development-armada/77119f87-2558-4694-8dc0-289060593199.jsonl')):
    o=json.loads(l)
    if o.get('type')=='api-request-shape':
        tt=o['shape']['tools'];print(sorted(((x.get('name'),len(json.dumps(x))) for x in tt),key=lambda x:-x[1]));break
p=os.path.realpath(os.popen('which claude').read().strip());f=open(p,'rb');m=mmap.mmap(f.fileno(),0,access=mmap.ACCESS_READ)
for k in[b'ENABLE_CLAUDEAI_MCP_SERVERS',b'CLAUDE_CODE_DISABLE_AUTO_MEMORY',b'skillListingBudgetFraction',b'SLASH_COMMAND_TOOL_CHAR_BUDGET',b'CLAUDE_CODE_SIMPLE',b'skillListingMaxDescChars',b'skillOverrides']:
    print(k.decode(),m.find(k)>=0)
i=m.find(b'triggerToolUses');j=0
idx=[]
s=0
while True:
    s=m.find(b'triggerToolUses',s)
    if s<0 or len(idx)>4:break
    idx.append(s);s+=20
print(idx)
for s in idx[1:3]:print(m[s-500:s+400].decode('utf8','replace').replace('\n',' ')[:900]);print('--')
for k in[b'SLEEP_COMPACT_AFTER_MS',b'COLD_COMPACT']:
    s=0;c=0
    while c<3:
        s=m.find(k,s)
        if s<0:break
        t=m[s-160:s+220].decode('utf8','replace')
        if 'process.env' in t or '.env.' in t or 'env' in t[:160]:
            print(k.decode(),t.replace('\n',' '));print('--')
        s+=20;c+=1
