#!/usr/bin/env python3
# Where the tokens went, from Claude Code transcripts. Streams; one process, low priority.
import json,os,re,sys,collections,time,pickle
ROOT=os.path.expanduser('~/.claude/projects')
DAYS=int(sys.argv[1]) if len(sys.argv)>1 else 14
now=time.time(); since=now-DAYS*86400
SINCE_ISO=time.strftime('%Y-%m-%dT%H:%M:%S',time.gmtime(since))
PRICE={'opus':5.0,'sonnet':3.0,'haiku':1.0,'fable':5.0}
def mprice(m):
    m=(m or '').lower()
    for k,v in PRICE.items():
        if k in m: return v
    return 3.0
CHARS=3.6
D=lambda:collections.defaultdict(float)
tot=collections.defaultdict(D)     # kind -> field -> value
bucket=collections.defaultdict(D)  # kind -> category -> cost
trigger=collections.defaultdict(D) # kind -> trigger -> cost
trigger_n=collections.defaultdict(D)
ctxband=collections.defaultdict(D)
shapes={}
threads=[]
reads=[]   # (session, thread, kind, path, whole, tokens, carry_cost, edited?, repeat?)
bash=collections.defaultdict(lambda:[0,0.0,0.0]) # head -> count,tokens,carry
grep_t=collections.defaultdict(lambda:[0,0.0,0.0])
tools_seen=collections.defaultdict(lambda:[0,0.0,0.0])
files_done=0
def norm(p):
    if not p: return ''
    p=re.sub(r'^.*?/Development/armada/(\.armada/(slots|preview)/[^/]+/)?(\.armada/slots/[^/]+/)?','',p)
    p=re.sub(r'^/private/tmp/[^/]+/','tmp/',p)
    return p
def textlen(c):
    if isinstance(c,str): return len(c)
    if isinstance(c,list):
        n=0
        for b in c:
            if isinstance(b,dict):
                if b.get('type')=='text': n+=len(b.get('text',''))
                elif b.get('type')=='tool_result': n+=textlen(b.get('content'))
                elif b.get('type')=='image': n+=4000
                elif b.get('type')=='tool_reference': n+=0
                else: n+=len(json.dumps(b))
            else: n+=len(str(b))
        return n
    return 0
def classify_user(text):
    t=text.lstrip()
    if t.startswith('Fleet:') or t.startswith('<fleet') or 'Fleet:' in t[:40]: return 'fleet-notice'
    if t.startswith('<task-notification'): return 'task-notification'
    if t.startswith('<system-reminder'): return 'system-reminder'
    if t.startswith('<command-') or t.startswith('<local-command'): return 'slash'
    if 'Base directory for this skill' in t[:200]: return 'skill-body'
    return 'person'
def bash_head(cmd):
    cmd=re.sub(r'^\s*cd\s+\S+\s*(&&|;)\s*','',cmd or '').strip()
    w=cmd.split()
    if not w: return '?'
    h=w[0]
    if h in('armada','gh','git','cargo','bd','npm','npx','pnpm','python3','scripts/preview'):
        return ' '.join(w[:2])
    return h
def thread_kind(path,first_user,entry):
    if '/subagents/' in path: return 'subagent'
    if '-private-var-folders' in path: return 'bg-temp'
    if '-private-tmp' in path: return 'bg-tmp'
    return 'main'
def process(path):
    global files_done
    sub='/subagents/' in path
    sess=os.path.basename(os.path.dirname(os.path.dirname(path))) if sub else os.path.basename(path)[:-6]
    kind=thread_kind(path,None,None)
    seen={}    # msg id -> usage entry index
    events=[]  # ordered stream
    agent_type=None
    meta=path[:-6]+'.meta.json'
    if sub and os.path.exists(meta):
        try: agent_type=json.load(open(meta)).get('agentType')
        except Exception: pass
    with open(path,errors='replace') as f:
        for line in f:
            if '"type":"' not in line[:400] and '"type": "' not in line: pass
            try:o=json.loads(line)
            except Exception: continue
            t=o.get('type')
            ts=o.get('timestamp','')
            if ts and ts<SINCE_ISO and t in('assistant','user'): continue
            if t=='assistant':
                m=o.get('message',{})
                mid=m.get('id')
                u=m.get('usage') or {}
                if mid in seen: e=seen[mid]
                else:
                    e={'k':'req','ts':ts,'model':m.get('model'),'u':u,'tools':[],'text':0,'sc':o.get('isSidechain')}
                    seen[mid]=e; events.append(e)
                e['u']=u or e['u']
                for b in m.get('content') or []:
                    if not isinstance(b,dict): continue
                    bt=b.get('type')
                    if bt=='text': e['text']+=len(b.get('text',''))
                    elif bt=='tool_use':
                        e['tools'].append((b.get('id'),b.get('name'),b.get('input') or {}))
                        e['text']+=len(json.dumps(b.get('input') or {}))
            elif t=='user':
                m=o.get('message',{}); c=m.get('content')
                if isinstance(c,list) and any(isinstance(b,dict) and b.get('type')=='tool_result' for b in c):
                    for b in c:
                        if isinstance(b,dict) and b.get('type')=='tool_result':
                            events.append({'k':'res','id':b.get('tool_use_id'),'n':textlen(b.get('content'))})
                        elif isinstance(b,dict): events.append({'k':'usr','cat':'system-reminder','n':textlen([b])})
                else:
                    txt=c if isinstance(c,str) else ''.join(b.get('text','') for b in (c or []) if isinstance(b,dict))
                    events.append({'k':'usr','cat':classify_user(txt),'n':textlen(c),'head':txt[:80]})
            elif t=='attachment':
                a=o.get('attachment') or {}
                events.append({'k':'att','cat':'att:'+str(a.get('type')),'n':len(json.dumps(a))})
            elif t=='system' and o.get('subtype')=='compact_boundary':
                events.append({'k':'compact'})
            elif t=='api-request-shape':
                s=o.get('shape') or {}
                h=o.get('shapeHash')
                if h not in shapes:
                    shapes[h]={'sys':sum(len(x.get('text','')) for x in s.get('system',[])),'tools':len(json.dumps(s.get('tools',[]))),'n':0,'names':[x.get('name') for x in s.get('tools',[])],'parts':[len(x.get('text','')) for x in s.get('system',[])],'kinds':collections.Counter()}
                shapes[h]['n']+=1; shapes[h]['kinds'][kind]+=1
                events.append({'k':'shape','h':h})
    # walk
    live=collections.defaultdict(float)
    pending={} # tool id -> (name,input)
    added=collections.defaultdict(float)
    prevC=None; nreq=0; maxC=0; cost_thread=0.0; last_trigger='start'
    first_ctx=None; first_ts=None; last_ts=None; model=None; cold=0; coldcost=0.0
    items=[]   # reads live: dict(rec) for carry
    live_reads=[]
    live_bash=[];live_tools=[]
    shape_changes=0; cur_shape=None; tcalls=0
    edited=set()
    for e in events:
        k=e['k']
        if k=='shape':
            if cur_shape and e['h']!=cur_shape: shape_changes+=1
            cur_shape=e['h']
        elif k=='compact':
            pass
        elif k=='usr':
            tok=e['n']/CHARS; live[e['cat']]+=tok; added[e['cat']]+=tok
            last_trigger=e['cat']
        elif k=='att':
            tok=e['n']/CHARS; live[e['cat']]+=tok; added[e['cat']]+=tok
        elif k=='res':
            nm,inp=pending.get(e['id'],('?',{}))
            cat='tool:'+(nm if not (nm or '').startswith('mcp__') else re.sub(r'^mcp__([^_]+(?:[-_][a-z]+)*?)__.*',r'mcp:\1',nm))
            if nm and nm.startswith('mcp__armada-fleet__'): cat='tool:fleet-act'
            elif nm and 'playwright' in nm: cat='tool:browser'
            elif nm and 'gitnexus' in nm: cat='tool:gitnexus'
            tok=e['n']/CHARS
            if nm=='Bash' and 'armada job' in (inp.get('command') or ''): cat='tool:fleet-act'
            live[cat]+=tok; added[cat]+=tok
            rec=[0.0,tok]; 
            if nm=='Read':
                r={'p':norm(inp.get('file_path')),'whole':inp.get('offset') is None and inp.get('limit') is None,'off':inp.get('offset'),'lim':inp.get('limit'),'tok':tok,'carry':0.0,'sess':sess,'thr':path,'kind':kind,'ts':e.get('ts')}
                reads.append(r); live_reads.append(r)
            elif nm=='Bash':
                r={'h':bash_head(inp.get('command')),'tok':tok,'carry':0.0}
                live_bash.append(r)
            else:
                r={'h':nm,'tok':tok,'carry':0.0}
                live_tools.append(r)
            last_trigger=cat
        elif k=='req':
            u=e['u'] or {}
            inp=u.get('input_tokens',0) or 0; cr=u.get('cache_read_input_tokens',0) or 0
            cw=u.get('cache_creation_input_tokens',0) or 0; out=u.get('output_tokens',0) or 0
            ccd=u.get('cache_creation') or {}
            w5=ccd.get('ephemeral_5m_input_tokens'); w1=ccd.get('ephemeral_1h_input_tokens')
            if w5 is None and w1 is None: w5=cw; w1=0
            w5=w5 or 0; w1=w1 or 0
            C=inp+cr+cw
            if C==0 and out==0: continue
            P=mprice(e['model'])/1e6
            cost_read=cr*0.1*P; cost_write=(w5*1.25+w1*2.0+inp)*P; cost_out=out*5*P
            T=tot[kind]
            T['requests']+=1;T['input']+=inp;T['cache_read']+=cr;T['w5']+=w5;T['w1']+=w1;T['output']+=out
            T['c_read']+=cost_read;T['c_write']+=cost_write;T['c_out']+=cost_out
            T['thinking']+=(u.get('output_tokens_details') or {}).get('thinking_tokens',0) or 0
            mk=(e['model'] or '?')
            tot['model:'+mk]['cost']+=cost_read+cost_write+cost_out; tot['model:'+mk]['requests']+=1
            cost=cost_read+cost_write+cost_out
            cost_thread+=cost
            band='<50k' if C<50000 else '50-100k' if C<100000 else '100-200k' if C<200000 else '200-300k' if C<300000 else '300-400k' if C<400000 else '400k+'
            ctxband[kind][band]+=cost
            ctxband[kind+':n'][band]+=1
            is_cold=prevC is not None and cr<0.5*prevC and C>0.7*prevC and cw>20000
            is_compact=prevC is not None and C<0.6*prevC
            if nreq==0:
                first_ctx=C; first_ts=e['ts']; model=e['model']
                bucket[kind]['start-up write']+=cost_write
                live.clear(); added.clear()
                live['baseline']=C
                # reads in the first messages were zero
            else:
                if is_compact:
                    live.clear(); added.clear(); live['after-compaction']=C
                    live_reads=[];live_bash=[];live_tools=[]
                    bucket[kind]['compaction rewrite']+=cost_write
                elif is_cold:
                    cold+=1; coldcost+=cost_write
                    bucket[kind]['cold cache rewrite']+=cost_write
                else:
                    s=sum(added.values())
                    if s>0:
                        for c,v in added.items(): bucket[kind][c+' (write)']+=cost_write*v/s
                    else: bucket[kind]['unattributed (write)']+=cost_write
                added.clear()
                # read cost shared over live
                s=sum(live.values())
                gap=max(0.0,C-s)
                if gap>0: live['unattributed']=live.get('unattributed',0)+gap
                s=sum(live.values()) or 1
                scale=C/s
                for c,v in live.items(): bucket[kind][c+' (carried)']+=cost_read*v/s
                unit=cost_read/s if s else 0
                for r in live_reads: r['carry']+=r['tok']*unit
                for r in live_bash: r['carry']+=r['tok']*unit
                for r in live_tools: r['carry']+=r['tok']*unit
            bucket[kind]['output']+=cost_out
            trigger[kind][last_trigger]+=cost
            trigger_n[kind][last_trigger]+=1
            # assistant's own content joins the context
            tok=e['text']/CHARS
            live['assistant text+calls']+=tok; added['assistant text+calls']+=tok
            for tid,nm,inpt in e['tools']:
                pending[tid]=(nm,inpt); tcalls+=1
                if nm in('Edit','Write','MultiEdit'): edited.add(norm(inpt.get('file_path')))
            prevC=C; maxC=max(maxC,C); nreq+=1; last_ts=e['ts']
    for r in live_bash:
        pass
    # flush per-thread carry into tallies
    # (carry was accumulated on the records themselves)
    for r in reads:
        if r['thr']==path: r['edited']=r['p'] in edited
    threads.append({'path':path,'sess':sess,'kind':kind,'agent':agent_type,'req':nreq,'first':first_ctx,'max':maxC,'cost':cost_thread,'model':model,'cold':cold,'coldcost':coldcost,'shapechg':shape_changes,'tcalls':tcalls,'ts':first_ts,'end':last_ts})
    files_done+=1
    return live_bash,live_tools
paths=[]
for d,_,fs in os.walk(ROOT):
    for fn in fs:
        if fn.endswith('.jsonl'):
            p=os.path.join(d,fn)
            try:
                if os.path.getmtime(p)>=since: paths.append(p)
            except OSError: pass
paths.sort()
print('files',len(paths),file=sys.stderr)
for i,p in enumerate(paths):
    try: 
        b,t=process(p)
        for r in b:
            x=bash[r['h']]; x[0]+=1;x[1]+=r['tok'];x[2]+=r['carry']
        for r in t:
            x=tools_seen[r['h']]; x[0]+=1;x[1]+=r['tok'];x[2]+=r['carry']
    except Exception as ex:
        print('ERR',p,ex,file=sys.stderr)
    if i%2000==0: print(i,file=sys.stderr,flush=True)
out=os.path.join(os.path.dirname(__file__),'data.pkl')
for s in shapes.values(): s['kinds']=dict(s['kinds'])
pickle.dump({'tot':{k:dict(v) for k,v in tot.items()},'bucket':{k:dict(v) for k,v in bucket.items()},'trigger':{k:dict(v) for k,v in trigger.items()},'trigger_n':{k:dict(v) for k,v in trigger_n.items()},'ctxband':{k:dict(v) for k,v in ctxband.items()},'shapes':shapes,'threads':threads,'reads':reads,'bash':dict(bash),'tools':dict(tools_seen)},open(out,'wb'))
print('done',out,file=sys.stderr)
