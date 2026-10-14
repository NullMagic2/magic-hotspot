#!/usr/bin/python3
# Module: magic_hotspot.tests.obex
# Purpose: Verify real BlueZ OBEX transfers across two virtual Bluetooth controllers.
# Created: 2026-10-13
# Architecture: Private session bus/client, btvirt radios, bounded RFCOMM test peer.
# This bypasses magic-usb and physical USB. Run as root under dbus-run-session.
import argparse, hashlib, json, os, pathlib, socket, struct, subprocess, threading, time
import gi
from gi.repository import Gio, GLib

parser=argparse.ArgumentParser()
parser.add_argument('--btvirt',required=True)
parser.add_argument('--output',required=True)
args=parser.parse_args()
assert os.geteuid()==0, 'Virtual HCI controllers require root'
assert os.environ.get('DBUS_SESSION_BUS_ADDRESS'), 'Use a private dbus-run-session'
root=pathlib.Path(args.output);root.mkdir(parents=True,exist_ok=True)
children=[];logs=[]
def spawn(argv,name,env=None):
 log=open(root/name,'w');logs.append(log)
 p=subprocess.Popen(argv,stdout=log,stderr=log,env=env);children.append(p);return p
def controllers():
 return {p.name:p for p in pathlib.Path('/sys/class/bluetooth').glob('hci*') if ':' not in p.name}
def run(argv):
 return subprocess.run(argv,check=True,capture_output=True,text=True,timeout=8).stdout
bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
def call(path,iface,method,args=None):
 return bus.call_sync('org.bluez.obex',path,iface,method,args,None,Gio.DBusCallFlags.NO_AUTO_START,15000,None).unpack()
def exact(s,n):
 b=b''
 while len(b)<n:
  data=s.recv(n-len(b))
  if not data: raise EOFError('peer disconnected mid-packet')
  b+=data
 return b
class Receiver:
 def __init__(self,address):
  self.data=bytearray();self.name=None;self.error=None;self.done=threading.Event()
  self.sock=socket.socket(socket.AF_BLUETOOTH,socket.SOCK_STREAM,socket.BTPROTO_RFCOMM)
  self.sock.settimeout(20)
  self.sock.setsockopt(274,4,struct.pack('BB',1,0)) # SOL_BLUETOOTH, BT_SECURITY_LOW
  self.sock.bind((address,0));self.sock.listen(1)
  self.channel=self.sock.getsockname()[1]
  self.thread=threading.Thread(target=self.receive,daemon=True);self.thread.start()
 def receive(self):
  try:
   with self.sock.accept()[0] as peer:
    peer.settimeout(15)
    while True:
     h=exact(peer,3);op=h[0];size=int.from_bytes(h[1:],'big')
     assert 3<=size<=65535
     body=exact(peer,size-3)
     if op==0x80: # OBEX Connect: version, flags, receive MTU
      peer.sendall(b'\xa0\x00\x07\x10\x00\x10\x00');continue
     if op==0x81:
      peer.sendall(b'\xa0\x00\x03');break
     assert op in (0x02,0x82), f'unexpected OBEX opcode {op:x}'
     offset=0
     while offset<len(body):
      kind=body[offset];typ=kind>>6
      if typ in (0,1):
       length=int.from_bytes(body[offset+1:offset+3],'big');assert length>=3
       value=body[offset+3:offset+length]
      else:
       length=2 if typ==2 else 5;value=body[offset+1:offset+length]
      assert offset+length<=len(body)
      if kind==1:self.name=value.decode('utf-16-be').rstrip('\0')
      if kind in (0x48,0x49):self.data.extend(value)
      offset+=length
     peer.sendall(bytes([0xa0 if op==0x82 else 0x90,0,3]))
     if op==0x82:self.done.set()
  except Exception as e:
   self.error=repr(e);self.done.set()
  finally:self.sock.close()

before=controllers();results=[]
try:
 emulator=spawn([args.btvirt,'-d','-l2'],'btvirt.log')
 deadline=time.monotonic()+8
 while time.monotonic()<deadline:
  added={k:v for k,v in controllers().items() if k not in before}
  if len(added)==2:break
  assert emulator.poll() is None,'btvirt exited'
  time.sleep(.1)
 assert len(added)==2,'two virtual controllers not created'
 addresses=[]
 for name,path in sorted(added.items()):
  # Refuse to touch any physical adapter, including an existing magic-usb VHCI.
  assert 'virtual' in str(path.resolve()),str(path.resolve())
  system=Gio.bus_get_sync(Gio.BusType.SYSTEM,None)
  adapter_path='/org/bluez/'+name
  for _ in range(60):
   try:
    props=system.call_sync('org.bluez',adapter_path,'org.freedesktop.DBus.Properties','GetAll',GLib.Variant('(s)',('org.bluez.Adapter1',)),None,Gio.DBusCallFlags.NONE,2000,None).unpack()[0]
    break
   except GLib.Error:time.sleep(.1)
  else:raise RuntimeError('virtual adapter not registered by BlueZ')
  address=props['Address']
  assert address.startswith('00:AA:'),address
  system.call_sync('org.bluez',adapter_path,'org.freedesktop.DBus.Properties','Set',GLib.Variant('(ssv)',('org.bluez.Adapter1','Powered',GLib.Variant('b',True))),None,Gio.DBusCallFlags.NONE,5000,None)
  system.call_sync('org.bluez',adapter_path,'org.freedesktop.DBus.Properties','Set',GLib.Variant('(ssv)',('org.bluez.Adapter1','Discoverable',GLib.Variant('b',True))),None,Gio.DBusCallFlags.NONE,5000,None)
  addresses.append(address)
 spawn(['/usr/bin/btmon','--index',sorted(added)[0][3:]],'virtual-hci.log')
 # Confirm only virtual-peer pairing events, never physical device requests.
 mgmt=socket.socket(socket.AF_BLUETOOTH,socket.SOCK_RAW,socket.BTPROTO_HCI)
 mgmt.bind((65535,3));mgmt.settimeout(.25)
 confirming=threading.Event();confirming.set()
 indices={int(name[3:]) for name in added}
 peers={bytes.fromhex(a.replace(':',''))[::-1] for a in addresses}
 def confirm_virtual():
  while confirming.is_set():
   try:packet=mgmt.recv(1024)
   except socket.timeout:continue
   except OSError:break
   if len(packet)<13:continue
   event,index,length=struct.unpack('<HHH',packet[:6])
   if event==0x000f and index in indices and packet[6:12] in peers:
    mgmt.send(struct.pack('<HHH',0x001c,index,7)+packet[6:13])
 confirmation=threading.Thread(target=confirm_virtual,daemon=True);confirmation.start()
 daemon=spawn(['/usr/libexec/bluetooth/obexd','--nodetach','--system-bus','--debug','--plugin=bluetooth,opp,filesystem','--root='+str(root)],'obexd.log',dict(os.environ,DBUS_SYSTEM_BUS_ADDRESS=os.environ['DBUS_SESSION_BUS_ADDRESS']))
 for _ in range(60):
  try:
   owned=bus.call_sync('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus','NameHasOwner',GLib.Variant('(s)',('org.bluez.obex',)),None,Gio.DBusCallFlags.NONE,1000,None).unpack()[0]
   assert daemon.poll() is None, 'private obexd exited'
   if owned:break
   time.sleep(.1)
  except GLib.Error:time.sleep(.1)
 else:raise RuntimeError('private obexd unavailable')
 statuses={}
 def changed(connection,sender,path,interface,signal,parameters):
  iface,properties,invalid=parameters.unpack()
  if iface=='org.bluez.obex.Transfer1' and 'Status' in properties:statuses[path]=properties['Status']
 subscription=bus.signal_subscribe('org.bluez.obex','org.freedesktop.DBus.Properties','PropertiesChanged',None,None,Gio.DBusSignalFlags.NONE,changed)
 context=GLib.MainContext.default()
 for direction,(source,target) in enumerate([addresses,addresses[::-1]]):
  for size in [0,127,1048576]:
   filename=f'test-{direction}-{size}-ação.bin'
   payload=(bytes(range(256))*((size+255)//256))[:size]
   file=root/filename;file.write_bytes(payload)
   receiver=Receiver(target)
   options={'Source':GLib.Variant('s',source),'Target':GLib.Variant('s','opp'),'Channel':GLib.Variant('y',receiver.channel)}
   session=call('/org/bluez/obex','org.bluez.obex.Client1','CreateSession',GLib.Variant('(sa{sv})',(target,options)))[0]
   try:
    transfer=call(session,'org.bluez.obex.ObjectPush1','SendFile',GLib.Variant('(s)',(str(file),)))[0]
    deadline=time.monotonic()+25
    while time.monotonic()<deadline:
     while context.pending():context.iteration(False)
     status=statuses.get(transfer,'queued')
     if status=='complete':break
     assert status!='error','BlueZ reported transfer error'
     time.sleep(.02)
    else:raise TimeoutError('OBEX transfer did not complete')
    assert receiver.done.wait(2) and receiver.error is None,receiver.error
    assert receiver.name==filename,(receiver.name,filename)
    assert bytes(receiver.data)==payload,'received bytes differ'
    digest=hashlib.sha256(receiver.data).hexdigest()
    result=dict(direction=direction,bytes=size,status=status,sha256=digest,filename_preserved=True)
    results.append(result);print('PASS',json.dumps(result),flush=True)
   finally:
    call('/org/bluez/obex','org.bluez.obex.Client1','RemoveSession',GLib.Variant('(o)',(session,)))
    receiver.thread.join(2)
 (root/'results.json').write_text(json.dumps({'scope':'Real BlueZ OBEX + kernel RFCOMM/L2CAP over btvirt; bypasses magic-usb and physical USB','transfers':results},indent=2)+'\n')
finally:
 if 'confirming' in globals():
  confirming.clear();mgmt.close();confirmation.join(1)
 for p in reversed(children):
  if p.poll() is None:p.terminate()
 for p in reversed(children):
  try:p.wait(timeout=5)
  except subprocess.TimeoutExpired:p.kill();p.wait()
 for f in logs:f.close()
 deadline=time.monotonic()+5
 while time.monotonic()<deadline and set(controllers())-set(before):time.sleep(.1)
 assert not (set(controllers())-set(before)),'virtual controller cleanup incomplete'
 print('Virtual controllers and private OBEX daemon cleaned up.',flush=True)
