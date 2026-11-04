#!/usr/bin/env python3
import json,os,re,sys,collections,time,pickle,hashlib,bisect
ROOT=os.path.expanduser('~/.claude/projects')
DAYS=14
since=time.time()-DAYS*86400
SINCE_ISO=time.strftime('%Y-%m-%dT%H:%M:%S',time.gmtime(since))
PRICE={'opus':5.0,'sonnet':3.0,'haiku':1.0,'fable':5.0}
def mprice(m):
    m=(m or '').lower()
    for k,v in PRICE.items():
        if k in m: return v
    return 3.0
CHARS=3.6
D=lambda:collections.defaultdict(float)
R=dict(bucket=collections.defaultdict(D),miss=collections.defaultdict(lambda:[0,0.0,0.0]),age=collections.defaultdict(D),
  calls=collections.defaultdict(lambda:collections.defaultdict(lambda:[0,0.0])),trig=collections.defaultdict(D),trign=collections.defaultdict(D),
  shape_trans=collections.defaultdict(lambda:[0,0.0]),tools_per=collections.defaultdict(D),preedit=collections.defaultdict(D),att=collections.defaultdict(lambda:[0,0.0]),hookadd=collections.defaultdict(lambda:[0,0.0]),
  guard=collections.Counter(),totals=collections.defaultdict(D),gaps=collections.defaultdict(D))
reads=[]; threads=[]
READLIKE=re.compile(r'^(sed|cat|head|tail|grep|rg|awk|nl|wc|ls|find|fd|git grep|git show|git log|git diff|gh |bd |tree|jq)\b')
def norm(p):
    if not p: return ''
    p=re.sub(r'^.*?/Development/armada/','',p)
    p=re.sub(r'^\.armada/(slots|preview)/[^/]+/','',p)
    p=re.sub(r'^\.armada/slots/[^/]+/','',p)
    return p
def strip_cd(cmd):
    return re.sub(r'^\s*(cd\s+\S+\s*(&&|;)\s*)+','',cmd or '').strip()
PATHRE=re.compile(r"((?:[\w./@-]+/)[\w.@-]+\.(?:rs|tsx?|md|css|toml|ya?ml|json|py|sh|html|mjs|cjs|lock))")
def bash_cat(cmd):
    c=strip_cd(cmd)
    if 'armada job' in c[:40]: return 'fleet'
    if re.match(r'(armada check|cargo|pnpm|npx|vitest|npm|scripts/|taskpolicy|nice)',c): return 'build/test'
    if re.match(r'(git (fetch|add|commit|push|merge|status|checkout|switch|rebase|worktree|branch|rm|mv)|gh pr (create|merge))',c): return 'git-act'
    if READLIKE.match(c): return 'read-like'
    if re.match(r'python3',c): return 'python'
    return 'other'
def textlen(c):
    if isinstance(c,str): return len(c)
    if isinstance(c,list):
        n=0
        for b in c:
            if isinstance(b,dict):
                if b.get('type')=='text': n+=len(b.get('text',''))
                elif b.get('type')=='tool_result': n+=textlen(b.get('content'))
                elif b.get('type')=='image': n+=5400
                elif b.get('type')=='tool_reference': n+=20
                else: n+=len(json.dumps(b))
            else: n+=len(str(b))
        return n
    return 0
def classify_user(t):
    t=t.lstrip()
    if t.startswith('Fleet:') or t[:60].find('Fleet:')>=0: return 'msg:fleet'
    if t.startswith('<task-notification'): return 'msg:task-notification'
    if t.startswith('<system-reminder'): return 'msg:reminder'
    if t.startswith('This session is being continued'): return 'msg:compaction-summary'
    if 'Base directory for this skill' in t[:200]: return 'msg:skill-body'
    return 'msg:person'
shape_sig={}
def process(path):
    sub='/subagents/' in path
    sess=os.path.basename(os.path.dirname(os.path.dirname(path))) if sub else os.path.basename(path)[:-6]
    kind='subagent' if sub else 'bg' if '-private-' in path else 'main'
    seen={};ev=[]
    with open(path,errors='replace') as f:
        for line in f:
            try:o=json.loads(line)
            except Exception: continue
            t=o.get('type');ts=o.get('timestamp','')
            if ts and ts<SINCE_ISO and t in('assistant','user'): continue
            if t=='assistant':
                m=o.get('message',{});mid=m.get('id');u=m.get('usage') or {}
                if mid in seen:e=seen[mid]
                else:
                    e={'k':'req','ts':ts,'model':m.get('model'),'u':u,'tools':[],'text':0}
                    seen[mid]=e;ev.append(e)
                e['u']=u or e['u']
                for b in m.get('content') or []:
                    if not isinstance(b,dict):continue
                    if b.get('type')=='text':e['text']+=len(b.get('text',''))
                    elif b.get('type')=='tool_use':
                        e['tools'].append((b.get('id'),b.get('name'),b.get('input') or {}));e['text']+=len(json.dumps(b.get('input') or {}))
            elif t=='user':
                c=o.get('message',{}).get('content')
                if isinstance(c,list) and any(isinstance(b,dict) and b.get('type')=='tool_result' for b in c):
                    for b in c:
                        if isinstance(b,dict) and b.get('type')=='tool_result':
                            cc=b.get('content');head=''
                            if isinstance(cc,str):head=cc[:40]
                            elif isinstance(cc,list) and cc and isinstance(cc[0],dict):head=(cc[0].get('text') or '')[:40]
                            ev.append({'k':'res','id':b.get('tool_use_id'),'n':textlen(cc),'head':head,'err':bool(b.get('is_error'))})
                        elif isinstance(b,dict):ev.append({'k':'usr','cat':'msg:reminder','n':textlen([b])})
                else:
                    txt=c if isinstance(c,str) else ''.join(b.get('text','') for b in (c or []) if isinstance(b,dict))
                    ev.append({'k':'usr','cat':classify_user(txt),'n':textlen(c)})
            elif t=='attachment':
                a=o.get('attachment') or {};r=o.get('rendered')
                if r:
                    n=sum(len(x.get('content','') if isinstance(x,dict) else str(x)) for x in r) if isinstance(r,list) else len(str(r))
                    ty=str(a.get('type'))
                    if ty=='hook_additional_context':
                        txt=json.dumps(r)
                        m=re.search(r'(\w+)(?::(\w+))? hook additional context: (\[?\w+)',txt)
                        hk=(m.group(1)+':'+(m.group(3) if m else '')) if m else 'other'
                        x=R['hookadd'][hk];x[0]+=1;x[1]+=n/CHARS
                    ev.append({'k':'att','cat':'att:'+ty,'n':n})
            elif t=='api-request-shape':
                s=o.get('shape') or {};h=o.get('shapeHash')
                if h not in shape_sig:
                    shape_sig[h]=(hashlib.md5(json.dumps(s.get('tools',[])).encode()).hexdigest(),hashlib.md5(json.dumps(s.get('system',[])).encode()).hexdigest())
                ev.append({'k':'shape','h':h})
            elif t=='api-request':
                ev.append({'k':'shape','h':o.get('shapeHash')})
    live=collections.defaultdict(float);added=collections.defaultdict(float);pending={};lres=[]
    prevC=None;prevTs=None;prevModel=None;nreq=0;maxC=0;cost_t=0.0;first=None;model=None;trig='start'
    cur=None;shape_at_prev=None;first_edit=None;reads_t={};tp=[];cost_before_edit=0.0;read_tok_before_edit=0.0;compactions=0
    edited=set();nreads=0
    for e in ev:
        k=e['k']
        if k=='shape': cur=e['h'] or cur
        elif k=='usr':
            tok=e['n']/CHARS;live[e['cat']]+=tok;added[e['cat']]+=tok;trig=e['cat']
        elif k=='att':
            tok=e['n']/CHARS;live[e['cat']]+=tok;added[e['cat']]+=tok
            x=R['att'][e['cat']];x[0]+=1;x[1]+=tok
        elif k=='res':
            nm,inp=pending.get(e['id'],('?',{}))
            nm=nm or '?'
            tok=e['n']/CHARS
            if nm=='Bash':
                bc=bash_cat(inp.get('command'));cat='Bash:'+bc
            elif nm.startswith('mcp__armada-fleet__'):cat='Fleet acts'
            elif 'playwright' in nm:cat='Browser'
            elif 'gitnexus' in nm:cat='gitnexus'
            elif nm.startswith('mcp__'):cat='other MCP'
            else:cat=nm
            cat='res:'+cat
            if nm=='Read' and e['head'].startswith('Not read:'): R['guard']['turned_away']+=1
            if nm=='Read':
                p=norm(inp.get('file_path'))
                whole=inp.get('offset') is None and inp.get('limit') is None
                rr={'via':'Read','p':p,'whole':whole,'off':inp.get('offset'),'lim':inp.get('limit'),'tok':tok,'carry':0.0,'sess':sess,'thr':path,'kind':kind,'i':nreq,'cost':0.0,'away':e['head'].startswith('Not read:')}
                reads.append(rr);lres.append(rr)
            elif nm=='Bash' and bash_cat(inp.get('command'))=='read-like':
                c=strip_cd(inp.get('command'))
                ps=PATHRE.findall(c)
                rr={'via':c.split()[0],'p':norm(ps[0]) if ps else '','whole':False,'off':None,'lim':None,'tok':tok,'carry':0.0,'sess':sess,'thr':path,'kind':kind,'i':nreq,'cost':0.0}
                reads.append(rr);lres.append(rr)
            else:
                lres.append({'tok':tok,'i':nreq,'carry':0.0,'cat':cat,'cost':0.0,'via':None})
            live[cat]+=tok;added[cat]+=tok;trig=cat
        elif k=='req':
            u=e['u'] or {}
            inp=u.get('input_tokens',0) or 0;cr=u.get('cache_read_input_tokens',0) or 0;cw=u.get('cache_creation_input_tokens',0) or 0;out=u.get('output_tokens',0) or 0
            ccd=u.get('cache_creation') or {};w5=ccd.get('ephemeral_5m_input_tokens');w1=ccd.get('ephemeral_1h_input_tokens')
            if w5 is None and w1 is None:w5=cw;w1=0
            w5=w5 or 0;w1=w1 or 0
            C=inp+cr+cw
            if C==0 and out==0:continue
            P=mprice(e['model'])/1e6
            wrate=2.0 if w1>=w5 and (w1 or w5) else 1.25
            c_read=cr*.1*P;c_write=(w5*1.25+w1*2.0+inp)*P;c_out=out*5*P;cost=c_read+c_write+c_out
            T=R['totals'][kind]
            T['req']+=1;T['cost']+=cost;T['read']+=c_read;T['write']+=c_write;T['out']+=c_out;T['w1']+=w1;T['w5']+=w5;T['cr']+=cr;T['outt']+=out;T['think']+=(u.get('output_tokens_details') or {}).get('thinking_tokens',0) or 0;T['in']+=inp
            T['m:'+str(e['model'])]+=cost
            R['trig'][kind][trig]+=cost;R['trign'][kind][trig]+=1
            b=R['bucket'][kind]
            b['~output']+=c_out
            def tsv(s):
                try:return time.mktime(time.strptime(s[:19],'%Y-%m-%dT%H:%M:%S'))
                except Exception:return None
            t_now=tsv(e['ts']) if e['ts'] else None
            gap=(t_now-prevTs) if (t_now and prevTs) else None
            comp=prevC is not None and C<0.6*prevC
            if nreq==0:
                first=C;model=e['model'];b['~start-up write']+=c_write
                live.clear();added.clear();live['~baseline']=C
                b['~baseline read at start']+=c_read
            else:
                ttl=3600 if kind!='subagent' else 300
                if comp:
                    compactions+=1
                    live.clear();added.clear();live['~after-compaction']=C
                    lres=[]
                    b['~compaction rewrite']+=c_write;b['res:compacted read']+=c_read
                else:
                    miss=max(0,prevC-cr)
                    misscost=miss*wrate*P
                    if miss>3000:
                        if gap is not None and gap>=ttl: why='ttl-expired'
                        elif cur!=shape_at_prev and shape_at_prev is not None:
                            a=shape_sig.get(cur);bb=shape_sig.get(shape_at_prev)
                            why='shape:'+('tools' if a and bb and a[0]!=bb[0] else '')+('+system' if a and bb and a[1]!=bb[1] else '')
                        elif e['model']!=prevModel: why='model-change'
                        else: why='other'
                        x=R['miss'][kind+'|'+why];x[0]+=1;x[1]+=miss;x[2]+=misscost
                        g='<5m' if (gap or 0)<300 else '5-60m' if (gap or 0)<3600 else '1-4h' if (gap or 0)<14400 else '4h+'
                        R['gaps'][kind+'|'+why][g]+=misscost
                    b['~cache miss rewrite']+=min(misscost,c_write)
                    wleft=max(0.0,c_write-misscost)
                    s=sum(added.values())
                    if s>0:
                        for c,v in added.items():b[c+' (write)']+=wleft*v/s
                    else:b['~unattributed (write)']+=wleft
                    added.clear()
                    s=sum(live.values());gp=max(0.0,C-s)
                    if gp>0:live['~unattributed']=gp+live.get('~unattributed',0)
                    s=sum(live.values()) or 1
                    for c,v in live.items():b[c]+=c_read*v/s
                    unit=c_read/s
                    for r in lres:
                        r['carry']+=r['tok']*unit
                        age=nreq-r['i']
                        band='<10' if age<10 else '10-30' if age<30 else '30-100' if age<100 else '100+'
                        R['age'][kind][band]+=r['tok']*unit
                    R['age'][kind]['liveresults']+=sum(r['tok'] for r in lres)*unit
                added.clear()
            if first_edit is None: cost_before_edit+=cost
            tok=e['text']/CHARS;live['assistant']+=tok;added['assistant']+=tok
            nt=len(e['tools'])
            R['tools_per'][kind][min(nt,5)]+=1
            names=[]
            for tid,nm,ip in e['tools']:
                pending[tid]=(nm,ip);names.append(nm)
                x=R['calls'][kind][nm if not nm.startswith('mcp__armada') else 'Fleet act'];x[0]+=1
                if nm in('Edit','Write','MultiEdit','NotebookEdit'):
                    if first_edit is None:first_edit=nreq
                    edited.add(norm(ip.get('file_path')))
            prevC=C;maxC=max(maxC,C);nreq+=1;prevTs=t_now or prevTs;prevModel=e['model'];shape_at_prev=cur
    for r in reads:
        if r.get('thr')==path and 'edited' not in r:
            r['edited']=r['p'] in edited
    threads.append(dict(path=path,sess=sess,kind=kind,req=nreq,first=first,max=maxC,cost=cost_t,model=model,comp=compactions,first_edit=first_edit,before_edit=cost_before_edit))
paths=[]
for d,_,fs in os.walk(ROOT):
    for fn in fs:
        if fn.endswith('.jsonl'):
            p=os.path.join(d,fn)
            try:
                if os.path.getmtime(p)>=since and '-private-var' not in p:paths.append(p)
            except OSError:pass
paths.sort()
print('files',len(paths),file=sys.stderr)
for i,p in enumerate(paths):
    try:process(p)
    except Exception as ex:print('ERR',p,repr(ex),file=sys.stderr)
def plain(x):
    if isinstance(x,collections.defaultdict) or isinstance(x,dict):return {k:plain(v) for k,v in x.items()}
    return x
out=os.path.join(os.path.dirname(os.path.abspath(__file__)),'data2.pkl')
pickle.dump(dict(R={k:plain(v) if not isinstance(v,collections.Counter) else dict(v) for k,v in R.items()},reads=reads,threads=threads),open(out,'wb'))
print('done',file=sys.stderr)
