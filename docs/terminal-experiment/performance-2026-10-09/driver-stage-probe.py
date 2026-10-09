#!/usr/bin/env python3
"""Standalone zsh driver stages; instrumentation evidence, no app/performance acceptance."""
import argparse, fcntl, hashlib, json, os, pty, select, signal, socket, subprocess, threading, time
from pathlib import Path
from secrets import token_hex

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--source-root',type=Path,required=True)
parser.add_argument('--helper',type=Path,required=True)
parser.add_argument('--output',type=Path,required=True)
parser.add_argument('--commands',type=int,default=6)
args=parser.parse_args()
assert 1 <= args.commands <= 20
args.helper=args.helper.resolve(strict=True)
args.output=args.output.absolute();args.output.mkdir(mode=0o700,parents=True,exist_ok=False)
source=(args.source_root/'src/experimental_terminal/lifecycle.rs').read_text()
a=source.index('Shell::Posix=>format!(r#"')+len('Shell::Posix=>format!(r#"')
b=source.index('"#, nonce = self.nonce)',a)
nonce,token=token_hex(16),token_hex(32)
helpers='edit(){ "$POTYI_TERM_CLIENT" --term-request "$POTYI_TERM_ADDRESS" "$POTYI_TERM_TOKEN" edit "$@"; }; view(){ "$POTYI_TERM_CLIENT" --term-request "$POTYI_TERM_ADDRESS" "$POTYI_TERM_TOKEN" view "$@"; }; cd(){ builtin cd "$@" && "$POTYI_TERM_CLIENT" --term-request "$POTYI_TERM_ADDRESS" "$POTYI_TERM_TOKEN" cwd; }; '
driver=source[a:b].replace('{{','{').replace('}}','}').replace('{nonce}',nonce).replace('{helpers}',helpers)
# zsh builtin timestamps avoid launching a diagnostic process at every stage.
# This fixed <=20-command fixture emits fewer than 400 short lines (<64 KiB).
logger="""zmodload zsh/datetime || exit $?
__potyi_trace() {
    printf '%s id=%s stage=%s\n' "$EPOCHREALTIME" "${__potyi_id:-0}" "$1" >> "$POTYI_DRIVER_TRACE_FILE"
}
"""
driver=logger+driver

def replace(old,new):
 global driver
 assert driver.count(old)==1,(old,driver.count(old))
 driver=driver.replace(old,new)

replace('    command -p stty sane -echo 2>/dev/null\n', '    __potyi_trace before_complete_stty\n    command -p stty sane -echo 2>/dev/null\n    __potyi_trace after_complete_stty\n')
client_line='    "$POTYI_TERM_CLIENT" --term-request "$POTYI_TERM_ADDRESS" "$POTYI_TERM_TOKEN" complete "$__potyi_id" "$1" "'+nonce+'"\n'
replace(client_line, '    __potyi_trace before_helper\n'+client_line+'    __potyi_helper_status=$?\n    __potyi_trace after_helper\n    return "$__potyi_helper_status"\n')
replace('    command -p mv ', '    __potyi_trace dispatch_read\n    command -p mv ')
replace('    __potyi_status=0\n','    __potyi_trace after_mv\n    __potyi_status=0\n')
replace('        command -p stty sane 2>/dev/null\n', '        __potyi_trace before_command_stty\n        command -p stty sane 2>/dev/null\n        __potyi_trace after_command_stty\n')
replace('            . "$POTYI_DRIVER_DIRECTORY/directory-$__potyi_id"\n            __potyi_status=$?\n','            . "$POTYI_DRIVER_DIRECTORY/directory-$__potyi_id"\n            __potyi_status=$?\n            __potyi_trace after_directory\n')
replace('                    __potyi_command=$(command -p cat ', '                    __potyi_trace before_cat\n                    __potyi_command=$(command -p cat ')
replace('                    __potyi_command=${__potyi_command%.}\n','                    __potyi_command=${__potyi_command%.}\n                    __potyi_trace before_child_exec\n')
replace('                ) < /dev/null\n                __potyi_status=$?\n','                ) < /dev/null\n                __potyi_status=$?\n                __potyi_trace after_child\n')
replace('    command -p rm -f ', '    __potyi_trace before_rm\n    command -p rm -f ')
replace('    __potyi_complete "$__potyi_status" || exit $?\n','    __potyi_trace after_rm\n    __potyi_complete "$__potyi_status" || exit $?\n')
(args.output/'driver').write_text(driver)
(args.output/'helpers').write_text(helpers)
server=socket.socket();server.bind(('127.0.0.1',0));server.listen();server.settimeout(.2)
stop=threading.Event();server_events=[]

def exact(stream,n):
 data=b''
 while len(data)<n:
  part=stream.recv(n-len(data))
  if not part:raise EOFError('short request')
  data+=part
 return data

def serve():
 while not stop.is_set():
  try: stream,_=server.accept()
  except socket.timeout:continue
  except OSError:return
  with stream:
   stream.settimeout(3)
   size=int.from_bytes(exact(stream,4),'big');assert size<=65536
   request=json.loads(exact(stream,size))
   assert request['token']==token and request['operation']=='complete'
   server_events.append({'time':time.time(),'id':request['command_id'],'stage':'server_request'})
   stream.sendall(b'OK')

def child_terminal():
 os.setsid();fcntl.ioctl(0,0x20007461,0) # Darwin TIOCSCTTY; macOS-only diagnostic.

master,slave=pty.openpty()
env=os.environ.copy();env.update(POTYI_DRIVER_SHELL='/bin/zsh',POTYI_DRIVER_DIRECTORY=str(args.output),POTYI_DRIVER_NONCE=nonce,POTYI_COMMAND_STDIN_NULL='1',POTYI_TERM_CLIENT=str(args.helper),POTYI_TERM_ADDRESS='127.0.0.1:'+str(server.getsockname()[1]),POTYI_TERM_TOKEN=token,POTYI_CHILD_BASH_ENV_SET='0',POTYI_CHILD_ENV_SET='0',POTYI_DRIVER_TRACE_FILE=str(args.output/'driver.log'),POTYI_TERM_TRACE_FILE=str(args.output/'client.log'))
for name in ['BASH_ENV','ENV','POTYI_COMMAND_SHELL']:env.pop(name,None)
quote=lambda path:"'"+str(path).replace("'","'\\''")+"'"
child=subprocess.Popen(['/bin/zsh','-f','-c','. '+quote(args.output/'driver')],stdin=slave,stdout=slave,stderr=slave,preexec_fn=child_terminal,env=env,cwd=args.output)
os.close(slave)
thread=threading.Thread(target=serve,daemon=True);thread.start()
parent_events=[];commands=[];buffer=b''

def until(marker):
 global buffer
 deadline=time.monotonic()+30
 while marker not in buffer:
  if time.monotonic()>deadline:raise TimeoutError(marker)
  if select.select([master],[],[],.2)[0]:
   buffer=(buffer+os.read(master,16384))[-131072:]
 buffer=buffer.split(marker,1)[1]

try:
 until(b'\x1b]777;potyi-ready\x07')
 for identifier in range(1,args.commands+1):
  started=time.monotonic();submitted=time.time()
  (args.output/f'command-{identifier}').write_text("printf '%s\\n' POTYI_STARTUP_DONE")
  (args.output/f'directory-{identifier}').write_text('builtin cd '+quote(args.output)+' || return $?\n')
  parent_events.append({'time':submitted,'id':identifier,'stage':'parent_submit'})
  os.write(master,(nonce+':'+str(identifier)+'\r').encode())
  until(('\x18\x1b]777;potyi-end;'+nonce+';'+str(identifier)+'\x07').encode())
  parent_events.append({'time':time.time(),'id':identifier,'stage':'parent_boundary_read'})
  commands.append({'id':identifier,'wall_ms':(time.monotonic()-started)*1000})
 # Let the final helper return be recorded; excluded from command wall_ms.
 time.sleep(.02)
finally:
 os.killpg(child.pid,signal.SIGKILL);child.wait();os.close(master);stop.set();server.close();thread.join(timeout=1)
trace_path=args.output/'driver.log';assert trace_path.stat().st_size<=65536
stages=[]
for line in trace_path.read_text().splitlines():
 timestamp,identifier,stage=line.split()
 stages.append({'time':float(timestamp),'id':int(identifier.split('=')[1]),'stage':stage.split('=')[1]})
all_events=sorted(parent_events+server_events+stages,key=lambda event:event['time'])
def digest(path):
 value=hashlib.sha256()
 with path.open('rb') as stream:
  while block:=stream.read(131072):value.update(block)
 return value.hexdigest()
report={'helper_sha256':digest(args.helper),'lifecycle_sha256':digest(args.source_root/'src/experimental_terminal/lifecycle.rs'),'bridge_sha256':digest(args.source_root/'src/experimental_terminal/bridge.rs'),'source_root':str(args.source_root.resolve()),'helper':str(args.helper),'commands':commands,'events':all_events,'clock':'Shared host wall seconds, command wall_ms monotonic','limitations':['Instrumented standalone zsh PTY, no application reader/SDL/layout stages.','Warm command excludes initial driver readiness.','Private mock ACK bridge; helper is exact real executable.','No cold-cache or stage timing acceptance claim.']}
(args.output/'results.json').write_text(json.dumps(report,indent=2)+'\n')
for command in commands:
 events=[event for event in all_events if event['id']==command['id']]
 intervals=[(a['stage']+' -> '+b['stage'],round((b['time']-a['time'])*1000,3)) for a,b in zip(events,events[1:])]
 print(json.dumps({'id':command['id'],'wall_ms':round(command['wall_ms'],3),'intervals':intervals}))
