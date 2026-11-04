#!/usr/bin/env python3
import json,os,re,sys,collections,time,pickle,random
ROOT=os.path.expanduser('~/.claude/projects')
since=time.time()-14*86400
SINCE_ISO=time.strftime('%Y-%m-%dT%H:%M:%S',time.gmtime(since))
recent=time.strftime('%Y-%m-%dT%H:%M:%S',time.gmtime(time.time()-5*86400))
def price(m):
    m=(m or '').lower()
    for k,v in {'opus':5.0,'sonnet':3.0,'haiku':1.0,'fable':5.0}.items():
        if k in m:return v
    return 3.0
READ=re.compile(r'^(sed|cat|head|tail|grep|rg|awk|nl|wc|ls|find|git grep|git show|git log|git diff|tree)\b')
def strip(c): return re.sub(r'^\s*(cd\s+\S+\s*(&&|;)\s*)+','',c or '').strip()
seqs=[];skills=collections.Counter();mcp=collections.Counter();tsq=collections.Counter();atypes=collections.Counter()
first=collections.defaultdict(list)
X=[];Y=[]
runs=collections.Counter();run_extra=collections.defaultdict(float);book=collections.defaultdict(lambda:[0,0.0])
system_parts={}
def is_readonly(tools):
    if not tools:return False
    for nm,ip in tools:
        if nm=='Read':continue
        if nm=='Bash' and READ.match(strip(ip.get('command'))):continue
        return False
    return True
def textlen(c):
    if isinstance(c,str):return len(c)
    n=0
    for b in c or []:
        if isinstance(b,dict):
            if b.get('type')=='text':n+=len(b.get('text',''))
            elif b.get('type')=='tool_result':n+=textlen(b.get('content'))
            elif b.get('type')=='image':n+=5400
            elif b.get('type')=='tool_reference':n+=20
            else:n+=len(json.dumps(b))
    return n
def process(path,sample):
    sub='/subagents/' in path;kind='subagent' if sub else 'main'
    reqs=[];seen={};pend={};cur_feat=None;att_first=collections.Counter();blob_first=None;nreq=0;usr_first=0;shape=None
    feats=[0.0]*7 # asst_vis,think,read,bash,otherres,att,user
    runlen=0;prev_C=None
    with open(path,errors='replace') as f:
        for line in f:
            try:o=json.loads(line)
            except Exception:continue
            t=o.get('type');ts=o.get('timestamp','')
            if t=='assistant':
                if ts and ts<SINCE_ISO:continue
                m=o.get('message',{});mid=m.get('id');u=m.get('usage') or {}
                if mid in seen:e=seen[mid]
                else:
                    e=dict(ts=ts,model=m.get('model'),u=u,tools=[],vis=0,feat=feats);feats=[0.0]*7
                    seen[mid]=e;reqs.append(e)
                e['u']=u or e['u']
                for b in m.get('content') or []:
                    if not isinstance(b,dict):continue
                    if b.get('type')=='text':e['vis']+=len(b.get('text',''))
                    elif b.get('type')=='tool_use':
                        nm=b.get('name');ip=b.get('input') or {}
                        e['tools'].append((nm,ip));e['vis']+=len(json.dumps(ip))
                        pend[b.get('id')]=(nm,ip)
                        if nm=='Skill':skills[ip.get('skill')]+=1
                        elif nm=='ToolSearch':tsq[(ip.get('query') or '')[:60]]+=1
                        elif nm=='Agent':atypes[ip.get('subagent_type') or 'none']+=1
                        elif nm and nm.startswith('mcp__'):mcp[nm.split('__')[1]]+=1
            elif t=='user':
                if ts and ts<SINCE_ISO:continue
                c=o.get('message',{}).get('content')
                if isinstance(c,list) and any(isinstance(b,dict) and b.get('type')=='tool_result' for b in c):
                    for b in c:
                        if isinstance(b,dict) and b.get('type')=='tool_result':
                            nm,ip=pend.get(b.get('tool_use_id'),('?',{}))
                            n=textlen(b.get('content'))
                            feats[2 if nm=='Read' else 3 if nm=='Bash' else 4]+=n
                        elif isinstance(b,dict):feats[5]+=textlen([b])
                else:
                    n=textlen(c);feats[6]+=n
                    if not reqs:usr_first+=n
            elif t=='attachment':
                r=o.get('rendered');a=o.get('attachment') or {}
                if r:
                    n=sum(len(x.get('content','') if isinstance(x,dict) else str(x)) for x in r) if isinstance(r,list) else len(str(r))
                    feats[5]+=n
                    if not reqs:att_first[str(a.get('type'))]+=n
            elif t=='api-request-shape' and shape is None:
                s=o.get('shape') or {};shape=([len(x.get('text','')) for x in s.get('system',[])],len(json.dumps(s.get('tools',[]))),len(s.get('tools',[])))
                if not sub and ts>recent and 'sys' not in system_parts:
                    system_parts['sys']=[x.get('text','') for x in s.get('system',[])]
    seq=[]
    for e in reqs:
        u=e['u'] or {}
        i=u.get('input_tokens',0) or 0;cr=u.get('cache_read_input_tokens',0) or 0;cw=u.get('cache_creation_input_tokens',0) or 0;out=u.get('output_tokens',0) or 0
        if i+cr+cw==0:continue
        try:tt=time.mktime(time.strptime(e['ts'][:19],'%Y-%m-%dT%H:%M:%S'))
        except Exception:tt=0
        seq.append((i+cr+cw,cr,cw,i,out,price(e['model']),tt))
    if seq:seqs.append((kind,path,seq))
    # regression + runs
    prevC=None;run=0
    for e in reqs:
        u=e['u'] or {}
        C=(u.get('input_tokens',0) or 0)+(u.get('cache_read_input_tokens',0) or 0)+(u.get('cache_creation_input_tokens',0) or 0)
        if C==0:continue
        cr=u.get('cache_read_input_tokens',0) or 0;cw=u.get('cache_creation_input_tokens',0) or 0;out=u.get('output_tokens',0) or 0
        P=price(e['model'])/1e6
        ttl=2.0 if kind=='main' else 1.25
        cost=(cr*.1+cw*ttl+out*5)*P
        names=[n for n,_ in e['tools']]
        ro=is_readonly(e['tools'])
        if ro:
            run+=1
            if run>1:run_extra[kind]+=cost;runs[kind+':extra']+=1
            runs[kind+':ro']+=1
        else:
            if run:runs[kind+':len%d'%min(run,10)]+=1
            run=0
        only=set(names)
        for tag,sel in (('TaskCreate/Update',{'TaskCreate','TaskUpdate','TaskList','TaskGet'}),('ToolSearch',{'ToolSearch'})):
            if only and only<=sel:
                x=book[kind+':'+tag];x[0]+=1;x[1]+=cost
        if only and all(n.startswith('mcp__armada-fleet') for n in only):
            x=book[kind+':Fleet act only'];x[0]+=1;x[1]+=cost
        if sample and prevC is not None and 0.6*prevC<C and cr>0.8*prevC:
            f=e['feat'];X.append([1.0,f[0],f[1],f[2],f[3],f[4],f[5],f[6]]);Y.append(C-prevC)
        prevC=C
        # shift: assistant features belong to the next delta
        vis=e['vis'];th=(u.get('output_tokens_details') or {}).get('thinking_tokens',0) or 0
        feats_next=None
        idx=reqs.index(e) if False else None
    # first request composition
    if reqs and shape and reqs[0]['ts']>recent:
        u=reqs[0]['u'] or {}
        C=(u.get('input_tokens',0) or 0)+(u.get('cache_read_input_tokens',0) or 0)+(u.get('cache_creation_input_tokens',0) or 0)
        first[kind].append(dict(C=C,att=dict(att_first),usr=usr_first,sys=shape[0],tools=shape[1],ntools=shape[2]))
paths=[]
for d,_,fs in os.walk(ROOT):
    for fn in fs:
        if fn.endswith('.jsonl') and '-private-' not in d:
            p=os.path.join(d,fn)
            if os.path.getmtime(p)>=since:paths.append(p)
paths.sort();random.seed(7)
for p in paths:
    try:process(p,random.random()<.25)
    except Exception as ex:print('ERR',p,repr(ex),file=sys.stderr)
pickle.dump(dict(seqs=seqs,skills=skills,mcp=mcp,tsq=tsq,atypes=atypes,first=dict(first),runs=runs,run_extra=dict(run_extra),book={k:v for k,v in book.items()},sys=system_parts),open(os.path.join(os.path.dirname(os.path.abspath(__file__)),'data3.pkl'),'wb'))
print('done',file=sys.stderr)
