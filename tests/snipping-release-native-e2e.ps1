# Reconstructed from the recorded temporary actual-release probe, not a claim of
# fresh execution. Opt-in added for durable storage; gate/input oracles retained.
# LegacyDual hardware contract retained: primary (0,0,2560,1440), secondary (2560,0,3840,2160).
# SingleDisplay is explicit opt-in: one enumerated monitor, exact outer AND client
# coverage, DPI-aligned 96-CSS square, independently work-area-derived preview.
# Does NOT observe native snip_ready IPC or arming. The visibility gate below
# follows show-after-all-decoded ordering, but show precedes emit_armed/armed=true.
# Its exact
# predicate may miss visible wrong-sized windows: a timeout is NOT an ACK failure.
# Clipboard: reads only DIB header metadata after preview, never pixel bytes.
param(
    [ValidateSet('LegacyDual','SingleDisplay')][string]$AcceptanceProfile='LegacyDual',
    [switch]$ProfileMetadataOnly
)
$ErrorActionPreference = 'Stop'
if (!$ProfileMetadataOnly -and $env:JASONSHELL_SNIP_RELEASE_E2E_TESTS -ne '1') {
    throw 'Explicit JASONSHELL_SNIP_RELEASE_E2E_TESTS=1 required; live shell/capture/clipboard consent also required.'
}
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Threading;
public static class SnipReleaseProbe {
 [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left,Top,Right,Bottom; }
 [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X,Y; }
 [StructLayout(LayoutKind.Sequential)] public struct MSG { public IntPtr hwnd; public uint message; public UIntPtr wParam; public IntPtr lParam; public uint time; public POINT pt; public uint extra; }
 public delegate bool EnumProc(IntPtr hwnd, IntPtr data);
 [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback,IntPtr data);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint pid);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr hwnd);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr GetPropW(IntPtr hwnd,string name);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd,out RECT rect);
 [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd,out RECT rect);
 [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd,ref POINT point);
 [DllImport("user32.dll")] public static extern IntPtr GetThreadDpiAwarenessContext();
 [DllImport("user32.dll")] public static extern bool AreDpiAwarenessContextsEqual(IntPtr a,IntPtr b);
 [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetClassNameW(IntPtr hwnd,System.Text.StringBuilder value,int capacity);
 [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr hwnd,uint command);
 [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtrW(IntPtr hwnd,int index);
 [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr hwnd,uint attribute,out RECT rect,uint bytes);
 public static long[] FailureFrameMetadata(IntPtr hwnd){
  RECT frame;int result=DwmGetWindowAttribute(hwnd,9,out frame,(uint)Marshal.SizeOf<RECT>());
  return new long[]{GetWindowLongPtrW(hwnd,-16).ToInt64(),GetWindowLongPtrW(hwnd,-20).ToInt64(),result,frame.Left,frame.Top,frame.Right-frame.Left,frame.Bottom-frame.Top};
 }
 [StructLayout(LayoutKind.Sequential)] public struct MONITORINFO { public uint size; public RECT monitor,work; public uint flags; }
 public delegate bool MonitorProc(IntPtr monitor,IntPtr dc,IntPtr rect,IntPtr data);
 [DllImport("user32.dll")] public static extern bool EnumDisplayMonitors(IntPtr dc,IntPtr clip,MonitorProc callback,IntPtr data);
 [DllImport("user32.dll")] public static extern bool GetMonitorInfoW(IntPtr monitor,ref MONITORINFO info);
 [DllImport("shcore.dll")] public static extern int GetDpiForMonitor(IntPtr monitor,int type,out uint x,out uint y);
 public static int[][] MonitorRows(){
  if(!AreDpiAwarenessContextsEqual(GetThreadDpiAwarenessContext(),new IntPtr(-4)))throw new Exception("monitor metadata requires PMv2");
  var rows=new List<int[]>();bool failed=false;
  if(!EnumDisplayMonitors(IntPtr.Zero,IntPtr.Zero,(h,dc,r,data)=>{
   var info=new MONITORINFO{size=(uint)Marshal.SizeOf<MONITORINFO>()};uint x,y;
   if(rows.Count>=32||!GetMonitorInfoW(h,ref info)||GetDpiForMonitor(h,0,out x,out y)<0||x==0||x!=y||x>int.MaxValue){failed=true;return false;}
   rows.Add(new[]{info.monitor.Left,info.monitor.Top,info.monitor.Right-info.monitor.Left,info.monitor.Bottom-info.monitor.Top,
    info.work.Left,info.work.Top,info.work.Right-info.work.Left,info.work.Bottom-info.work.Top,(int)x,(int)y});return true;
  },IntPtr.Zero)||failed||rows.Count==0)throw new Exception("monitor metadata unavailable");
  return rows.ToArray();
 }
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr hwnd,uint msg,IntPtr w,IntPtr l);
 [DllImport("user32.dll")] public static extern int GetMessageW(out MSG msg,IntPtr hwnd,uint min,uint max);
 [DllImport("user32.dll")] public static extern void PostQuitMessage(int code);
 [DllImport("user32.dll")] public static extern bool DestroyWindow(IntPtr hwnd);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr DefWindowProcW(IntPtr hwnd,uint msg,IntPtr w,IntPtr l);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr GetModuleHandleW(string name);
 [DllImport("kernel32.dll")] public static extern uint GetCurrentProcessId();
 [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
 [UnmanagedFunctionPointer(CallingConvention.Winapi)] public delegate IntPtr WindowProc(IntPtr hwnd,uint msg,IntPtr w,IntPtr l);
 [UnmanagedFunctionPointer(CallingConvention.Winapi)] public delegate void WinEventProc(IntPtr hook,uint evt,IntPtr hwnd,int obj,int child,uint thread,uint time);
 [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] public struct WNDCLASSEX {
  public uint cbSize,style; public WindowProc proc; public int classExtra,windowExtra;
  public IntPtr instance,icon,cursor,brush,menu; [MarshalAs(UnmanagedType.LPWStr)] public string name; public IntPtr smallIcon;
 }
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern ushort RegisterClassExW(ref WNDCLASSEX cls);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern bool UnregisterClassW(string name,IntPtr instance);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr CreateWindowExW(uint ex,string cls,string title,uint style,int x,int y,int width,int height,IntPtr parent,IntPtr menu,IntPtr instance,IntPtr data);
 [DllImport("user32.dll")] public static extern IntPtr SetWinEventHook(uint first,uint last,IntPtr module,WinEventProc callback,uint pid,uint thread,uint flags);
 [DllImport("user32.dll")] public static extern bool UnhookWinEvent(IntPtr hook);
 [DllImport("user32.dll")] public static extern bool PeekMessageW(out MSG msg,IntPtr hwnd,uint min,uint max,uint flags);
 [DllImport("user32.dll")] public static extern bool TranslateMessage(ref MSG msg);
 [DllImport("user32.dll")] public static extern IntPtr DispatchMessageW(ref MSG msg);
 [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT point);
 [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int key);
 [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
 [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr hwnd,uint flag);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int width,int height,uint flags);
 [DllImport("user32.dll")] public static extern void mouse_event(uint flags,uint dx,uint dy,uint data,UIntPtr extra);
 [DllImport("user32.dll")] public static extern bool GetWindowDisplayAffinity(IntPtr hwnd,out uint affinity);
 [DllImport("user32.dll")] public static extern uint GetClipboardSequenceNumber();
 [DllImport("user32.dll")] public static extern bool OpenClipboard(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern bool CloseClipboard();
 [DllImport("user32.dll")] public static extern IntPtr GetClipboardData(uint format);
 [DllImport("user32.dll")] public static extern bool IsClipboardFormatAvailable(uint format);
 [DllImport("kernel32.dll")] public static extern IntPtr GlobalLock(IntPtr handle);
 [DllImport("kernel32.dll")] public static extern bool GlobalUnlock(IntPtr handle);
 [DllImport("kernel32.dll")] public static extern UIntPtr GlobalSize(IntPtr handle);
 public static IntPtr[] Windows(int pid) { var list=new List<IntPtr>(); EnumWindows((h,d)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid&&IsWindowVisible(h))list.Add(h);return true;},IntPtr.Zero);return list.ToArray(); }
 public sealed class WindowIdentity { public long Hwnd,Root,Epoch; public uint Pid,Thread; public bool Live,Visible,ClassVerified; public int ClassCode=255; }
 public static WindowIdentity Identity(IntPtr h){
  bool live=IsWindow(h);uint pid;uint thread=GetWindowThreadProcessId(h,out pid);
  return new WindowIdentity{Hwnd=h.ToInt64(),Root=GetAncestor(h,2).ToInt64(),Pid=pid,Thread=thread,
   Epoch=GetPropW(h,"JasonShell.Snip.Instance").ToInt64(),Live=live&&IsWindow(h),Visible=IsWindowVisible(h)};
 }
 // Fixed metadata codes only. The observed raw class string NEVER leaves C#.
 // Code4 is Tao0.34.8's verified default in platform_impl/windows/mod.rs55;
 // this production NativeWindowPort builder does not override window_classname.
 public static int ClassCode(string value){switch(value){
  case "IME":return 1;case "MSCTFIME UI":return 2;case "CicMarshalWndClass":return 3;
  case "Window Class":return 4;case "tooltips_class32":return 5;case "SysShadow":return 6;case "#32768":return 7;
  default:return 0;
 }}
 static bool SameSnapshot(WindowIdentity a,WindowIdentity b){return a.Live&&b.Live&&a.Hwnd==b.Hwnd&&a.Root==b.Root&&a.Pid==b.Pid&&a.Thread==b.Thread&&a.Epoch==b.Epoch&&a.Visible==b.Visible;}
 public static WindowIdentity[] VerifyAddedRootClasses(WindowIdentity[] original,WindowIdentity[] inventory,IntPtr protectedBar,long protectedEpoch){
  var known=new HashSet<long>();foreach(var row in original)known.Add(row.Hwnd);
  var facts=new List<WindowIdentity>();var bar=Identity(protectedBar);
  foreach(var row in inventory){if(known.Contains(row.Hwnd))continue;
   var first=Identity(new IntPtr(row.Hwnd));var buffer=new System.Text.StringBuilder(256);
   int code=GetClassNameW(new IntPtr(row.Hwnd),buffer,buffer.Capacity)>0?ClassCode(buffer.ToString()):255;
   var last=Identity(new IntPtr(row.Hwnd));last.ClassCode=code;
   last.ClassVerified=SameSnapshot(row,first)&&SameSnapshot(first,last)&&bar.Live&&bar.Root==bar.Hwnd&&
    bar.Epoch==protectedEpoch&&protectedEpoch!=0&&bar.Pid==last.Pid;
   facts.Add(last);
  }return facts.ToArray();
 }
 public static long[] AddedRootMetadata(IntPtr hwnd,uint expectedPid,uint expectedThread,long expectedEpoch,IntPtr protectedBar,long protectedEpoch){
  var first=Identity(hwnd);var bar=Identity(protectedBar);
  if(!first.Live||first.Pid!=expectedPid||first.Thread!=expectedThread||first.Epoch!=expectedEpoch||first.Root!=first.Hwnd||
   !bar.Live||bar.Pid!=expectedPid||bar.Epoch!=protectedEpoch||protectedEpoch==0)return new long[]{0,255,0,0,0,0,0,0,0};
  var buffer=new System.Text.StringBuilder(256);int code=GetClassNameW(hwnd,buffer,buffer.Capacity)>0?ClassCode(buffer.ToString()):255;
  long style=GetWindowLongPtrW(hwnd,-16).ToInt64(),exstyle=GetWindowLongPtrW(hwnd,-20).ToInt64(),owner=GetWindow(hwnd,4).ToInt64();
  var last=Identity(hwnd);
  if(!last.Live||last.Pid!=first.Pid||last.Thread!=first.Thread||last.Epoch!=first.Epoch||last.Root!=first.Root)return new long[]{0,255,0,0,0,0,0,0,0};
  return new long[]{1,code,style,exstyle,owner,last.Thread,last.Thread==bar.Thread?1:0,last.Epoch,bar.Thread};
 }
 // All owned TOP-LEVEL roots, including hidden windows with no registry property.
 // A registry-only or visible-only list would miss an overlay during construction.
 public static WindowIdentity[] AllOwnedIdentities(int pid){
  var list=new List<WindowIdentity>();bool invalid=false;
  bool enumerated=EnumWindows((h,d)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid){
   var row=Identity(h);if(list.Count>=256||!row.Live||row.Pid!=pid||row.Root!=row.Hwnd){invalid=true;return false;}
   list.Add(row);
  }return true;},IntPtr.Zero);
  if(!enumerated||invalid)throw new Exception("owned inventory inconclusive");
  return list.ToArray();
 }
 public static IntPtr[] Registered(int pid) { var list=new List<IntPtr>(); EnumWindows((h,d)=>{uint p;GetWindowThreadProcessId(h,out p);if(p==pid&&GetPropW(h,"JasonShell.Snip.Instance")!=IntPtr.Zero)list.Add(h);return true;},IntPtr.Zero);return list.ToArray(); }
 public static int[] Bounds(IntPtr h) {RECT r;if(!GetWindowRect(h,out r))throw new Exception("bounds unavailable");return new[]{r.Left,r.Top,r.Right-r.Left,r.Bottom-r.Top};}
 public static int[] ClientBounds(IntPtr h) {RECT r;POINT p=new POINT();if(!GetClientRect(h,out r)||!ClientToScreen(h,ref p))throw new Exception("client bounds unavailable");return new[]{p.X,p.Y,r.Right-r.Left,r.Bottom-r.Top};}
 public static IntPtr At(int x,int y){return WindowFromPoint(new POINT{X=x,Y=y});}
 public static uint ProcessId(IntPtr hwnd){uint pid;GetWindowThreadProcessId(hwnd,out pid);return pid;}
 public static bool LeftIsDown(){return (GetAsyncKeyState(1)&0x8000)!=0;}
 public static bool CursorAt(int x,int y){POINT p;return GetCursorPos(out p)&&p.X==x&&p.Y==y;}
 public static bool CursorOver(IntPtr root){POINT p;return GetCursorPos(out p)&&GetAncestor(WindowFromPoint(p),2)==root;}
 // Separate responsive UI thread; windows are created only AFTER the shell bar
 // exists. Recorder receives OS WinEvents scoped to this process and filters to
 // these two exact HWND/thread identities. It is NOT the shell observer's ACK.
 static Thread fixtureThread; static readonly ManualResetEvent fixtureReady=new ManualResetEvent(false);
 static IntPtr neutralA,neutralB,focusHook; static uint fixtureThreadId; static int fixtureWindows,lastFocus;
 static long focusA,focusB; static bool fixtureFailed;
 static readonly WindowProc fixtureProc=FixtureMessage; static readonly WinEventProc focusProc=FixtureFocus;
 public static IntPtr NeutralA { get { return neutralA; } }
 public static IntPtr NeutralB { get { return neutralB; } }
 static bool OwnFixture(IntPtr h){uint pid;return h!=IntPtr.Zero&&GetWindowThreadProcessId(h,out pid)==fixtureThreadId&&pid==GetCurrentProcessId();}
 static IntPtr FixtureMessage(IntPtr h,uint msg,IntPtr w,IntPtr l){
  if(msg==16){DestroyWindow(h);return IntPtr.Zero;}
  if(msg==2&&(h==neutralA||h==neutralB)){if(Interlocked.Decrement(ref fixtureWindows)==0)PostQuitMessage(0);return IntPtr.Zero;}
  return DefWindowProcW(h,msg,w,l);
 }
 static void FixtureFocus(IntPtr hook,uint evt,IntPtr h,int obj,int child,uint thread,uint time){
  if(evt!=3||!OwnFixture(h))return;
  if(h==neutralA){Interlocked.Increment(ref focusA);Volatile.Write(ref lastFocus,1);}
  else if(h==neutralB){Interlocked.Increment(ref focusB);Volatile.Write(ref lastFocus,2);}
 }
 public static long NeutralFocusCount(IntPtr h){return h==neutralA?Interlocked.Read(ref focusA):h==neutralB?Interlocked.Read(ref focusB):-1;}
 public static int NeutralLastFocus(){return Volatile.Read(ref lastFocus);}
  public static IntPtr StartNeutralFixtures(){return StartNeutralFixturesAt(2600,300,250,500,2900,300,600,500);}
  public static IntPtr StartNeutralFixturesAt(int ax,int ay,int aw,int ah,int bx,int by,int bw,int bh){
  if(fixtureThread!=null)throw new Exception("neutral fixture already started");
  fixtureThread=new Thread(()=>{
   var prior=SetThreadDpiAwarenessContext(new IntPtr(-4));fixtureThreadId=GetCurrentThreadId();
   var instance=GetModuleHandleW(null);string cls="SnipOwnedNeutral"+GetCurrentProcessId();bool registered=false;
   try{
    var wc=new WNDCLASSEX{cbSize=(uint)Marshal.SizeOf<WNDCLASSEX>(),proc=fixtureProc,instance=instance,brush=new IntPtr(16),name=cls};
    if(RegisterClassExW(ref wc)==0)throw new Exception("neutral class registration failed");registered=true;
    focusHook=SetWinEventHook(3,3,IntPtr.Zero,focusProc,GetCurrentProcessId(),0,0);
    if(focusHook==IntPtr.Zero)throw new Exception("neutral recorder unavailable");
    fixtureWindows=2;
     neutralA=CreateWindowExW(0,cls,"Owned neutral A",0x90000000u,ax,ay,aw,ah,IntPtr.Zero,IntPtr.Zero,instance,IntPtr.Zero);
     neutralB=CreateWindowExW(0,cls,"Owned neutral B",0x90000000u,bx,by,bw,bh,IntPtr.Zero,IntPtr.Zero,instance,IntPtr.Zero);
    if(neutralA==IntPtr.Zero||neutralB==IntPtr.Zero)throw new Exception("neutral fixture creation failed");
    fixtureReady.Set();MSG msg;int read;while((read=GetMessageW(out msg,IntPtr.Zero,0,0))>0){TranslateMessage(ref msg);DispatchMessageW(ref msg);}
    if(read<0)fixtureFailed=true;
   }catch{fixtureFailed=true;}
   finally{
    if(OwnFixture(neutralA)&&IsWindow(neutralA))DestroyWindow(neutralA);
    if(OwnFixture(neutralB)&&IsWindow(neutralB))DestroyWindow(neutralB);
    if(focusHook!=IntPtr.Zero)UnhookWinEvent(focusHook);
    if(registered)UnregisterClassW(cls,instance);if(prior!=IntPtr.Zero)SetThreadDpiAwarenessContext(prior);fixtureReady.Set();
   }
  });fixtureThread.SetApartmentState(ApartmentState.STA);fixtureThread.IsBackground=true;fixtureThread.Start();
  if(!fixtureReady.WaitOne(5000)||fixtureFailed||!OwnFixture(neutralA)||!OwnFixture(neutralB))throw new Exception("responsive neutral fixture startup failed");
  return neutralB;
 }
 public static bool StopNeutralFixtures(){
  if(fixtureThread==null)return true;
  if(!fixtureReady.WaitOne(1000))return false;
  if(OwnFixture(neutralA)&&IsWindow(neutralA))PostMessageW(neutralA,16,IntPtr.Zero,IntPtr.Zero);
  if(OwnFixture(neutralB)&&IsWindow(neutralB))PostMessageW(neutralB,16,IntPtr.Zero,IntPtr.Zero);
  return fixtureThread.Join(5000)&&!IsWindow(neutralA)&&!IsWindow(neutralB);
 }
 public static void Pump(){MSG m;while(PeekMessageW(out m,IntPtr.Zero,0,0,1)){TranslateMessage(ref m);DispatchMessageW(ref m);}}
 public static int[] ClipboardHeader(){if(!OpenClipboard(IntPtr.Zero))return null;try{foreach(uint f in new uint[]{17,8}){var h=GetClipboardData(f);if(h==IntPtr.Zero)continue;var p=GlobalLock(h);if(p==IntPtr.Zero)continue;try{int size=Marshal.ReadInt32(p);if(size<40)return null;return new[]{(int)f,Marshal.ReadInt32(p,4),Math.Abs(Marshal.ReadInt32(p,8)),(int)(ushort)Marshal.ReadInt16(p,14)};}finally{GlobalUnlock(h);}}return null;}finally{CloseClipboard();}}
 // Separate diagnostic reader: only allowlisted image formats and <=40 header
 // bytes, guarded by GlobalSize. Never reads pixel data or other format names.
 // ClipboardHeader and all acceptance conditions remain unchanged.
 public static string ClipboardMetadata(){
  if(!OpenClipboard(IntPtr.Zero))return "open=false";
  try{
   var text=new System.Text.StringBuilder("open=true");
   text.Append(";CF_BITMAP(2).available=").Append(IsClipboardFormatAvailable(2));
   foreach(uint f in new uint[]{17,8}){
    string name=f==17?"CF_DIBV5(17)":"CF_DIB(8)";bool available=IsClipboardFormatAvailable(f);
    text.Append(';').Append(name).Append(".available=").Append(available);if(!available)continue;
    var h=GetClipboardData(f);text.Append(",getData=").Append(h!=IntPtr.Zero);if(h==IntPtr.Zero)continue;
    ulong allocation=GlobalSize(h).ToUInt64();text.Append(",allocationBytes=").Append(allocation);
    if(allocation<4){text.Append(",headerRead=false(reason=allocation-under-4)");continue;}
    var p=GlobalLock(h);text.Append(",lock=").Append(p!=IntPtr.Zero);if(p==IntPtr.Zero)continue;
    try{
     uint size=unchecked((uint)Marshal.ReadInt32(p));text.Append(",headerSize=").Append(size);
     bool bounded=size>=40&&size<=allocation&&allocation>=40;text.Append(",headerWithinAllocation=").Append(bounded);
     if(!bounded){text.Append(",headerRead=false");continue;}
     int width=Marshal.ReadInt32(p,4),height=Marshal.ReadInt32(p,8);
     text.Append(",headerRead=true,width=").Append(width).Append(",signedHeight=").Append(height)
       .Append(",orientation=").Append(height<0?"top-down":"bottom-up")
       .Append(",bpp=").Append((ushort)Marshal.ReadInt16(p,14))
       .Append(",compression=").Append(unchecked((uint)Marshal.ReadInt32(p,16)));
    }finally{GlobalUnlock(h);}
   }
   return text.ToString();
  }finally{CloseClipboard();}
 }
}
'@
# Pure independent acceptance math. Source: native.rs preview dimensions/workarea;
# SnipOverlaySurface client-minus-root -> snipSelection -> geometry logical*scale.
# No measured DOM coordinates are inferred/claimed by this profile constructor.
function New-SnipSingleProfile([int[]]$row) {
    if($row.Count -ne 10){throw 'single-profile-invalid-metadata-shape'}
    [long]$left=$row[0];[long]$top=$row[1];[long]$width=$row[2];[long]$height=$row[3]
    [long]$wl=$row[4];[long]$wt=$row[5];[long]$ww=$row[6];[long]$wh=$row[7];[long]$dpi=$row[8]
    if($width -le 0 -or $height -le 0 -or $width -gt 16384 -or $height -gt 16384 -or $width*$height -gt 33600000 -or
       $dpi -le 0 -or $dpi -ne $row[9] -or $ww -le 0 -or $wh -le 0 -or $wl -lt $left -or $wt -lt $top -or
       $wl+$ww -gt $left+$width -or $wt+$wh -gt $top+$height){throw 'single-profile-invalid-monitor-workarea-dpi'}
    $scale=$dpi/96.0
    [long]$pw=[Math]::Round(320.0*$scale,[MidpointRounding]::AwayFromZero)
    [long]$ph=[Math]::Round(264.0*$scale,[MidpointRounding]::AwayFromZero)
    if($pw -gt $ww -or $ph -gt $wh -or 2*$dpi -ge $width -or 2*$dpi -ge $height -or $ww -lt 512 -or $wh -lt 300){throw 'single-profile-insufficient-space'}
    $numbers=@(($left+$width),($top+$height),($wl+$ww),($wt+$wh),($left+$dpi),($top+$dpi),($left+2*$dpi),($top+2*$dpi))
    if(@($numbers|Where-Object {$_ -lt [int]::MinValue -or $_ -gt [int]::MaxValue}).Count){throw 'single-profile-coordinate-overflow'}
    [pscustomobject]@{
        Bounds=[int[]]@($left,$top,$width,$height); WorkArea=[int[]]@($wl,$wt,$ww,$wh); Dpi=[int]$dpi; Scale=$scale
        SelectionStart=[int[]]@(($left+$dpi),($top+$dpi)); SelectionEnd=[int[]]@(($left+2*$dpi),($top+2*$dpi))
        SelectionSize=[int]$dpi; Preview=[int[]]@(($wl+$ww-$pw),$wt,$pw,$ph)
        FixtureA=[int[]]@(($wl+16),($wt+80),160,160); FixtureB=[int[]]@(($wl+208),($wt+80),256,160)
        ClickA=[int[]]@(($wl+48),($wt+112)); ClickB=[int[]]@(($wl+240),($wt+112))
    }
}
function Read-SnipSingleProfile {
    $rows=[SnipReleaseProbe]::MonitorRows()
    if($rows.Length -ne 1){throw 'single-profile-requires-exactly-one-monitor'}
    New-SnipSingleProfile $rows[0]
}
if($ProfileMetadataOnly){
    if($AcceptanceProfile -ne 'SingleDisplay'){throw 'metadata-only-requires-single-display-profile'}
    $prior=[SnipReleaseProbe]::SetThreadDpiAwarenessContext([IntPtr](-4))
    try {
        $profile=Read-SnipSingleProfile
        "PROFILE metadata-only=true pmv2=true monitor=$($profile.Bounds -join ',') work=$($profile.WorkArea -join ',') dpi=$($profile.Dpi) predictedStart=$($profile.SelectionStart -join ',') predictedEnd=$($profile.SelectionEnd -join ',') expectedClipboard=$($profile.SelectionSize)x$($profile.SelectionSize) expectedPreview=$($profile.Preview -join ',') DOMobserved=false nativeJourneyRun=false"
    } finally {if($prior -ne [IntPtr]::Zero){[void][SnipReleaseProbe]::SetThreadDpiAwarenessContext($prior)}}
    exit 0
}
function Require($condition, [string]$message) { if (!$condition) { throw $message } }
function Wait-Stage([scriptblock]$condition, [string]$name, [int]$seconds=40) {
    # Exact historical clock: UTC wall time, not Stopwatch/monotonic. Native/UIA
    # calls are synchronous, so this is a nominal bound, not a hard watchdog.
    $end=[DateTime]::UtcNow.AddSeconds($seconds)
    do { [SnipReleaseProbe]::Pump(); $result=&$condition; if($result){return $result}; Start-Sleep -Milliseconds 60 } while([DateTime]::UtcNow -lt $end)
    throw "timeout:$name"
}
function Find-Name([IntPtr]$handle,[string]$name) {
    try {
        $root=[System.Windows.Automation.AutomationElement]::FromHandle($handle)
        return $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,$name))
    } catch { return $null }
}
function Invoke-Element($element) { ([System.Windows.Automation.InvokePattern]$element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)).Invoke() }
# Global input cannot be made atomic against concurrent human/window activity.
# Refuse an already-held physical left button. Revalidate identity and hit target
# at each input boundary; release only a press this script recorded as injected.
function Assert-InputTarget([IntPtr]$root,[uint32]$ownerPid,[int]$x,[int]$y,[IntPtr]$epoch=[IntPtr]::Zero) {
    Require ([SnipReleaseProbe]::IsWindow($root) -and [SnipReleaseProbe]::IsWindowVisible($root)) 'input-target-not-live-visible'
    Require ([SnipReleaseProbe]::ProcessId($root) -eq $ownerPid) 'input-target-owner-changed'
    if($epoch -ne [IntPtr]::Zero){Require ([SnipReleaseProbe]::GetPropW($root,'JasonShell.Snip.Instance') -eq $epoch) 'input-target-epoch-changed'}
    Require ([SnipReleaseProbe]::GetAncestor([SnipReleaseProbe]::At($x,$y),2) -eq $root) 'input-target-hit-test-mismatch'
}
function Start-InjectedLeft([IntPtr]$root,[uint32]$ownerPid,[int]$x,[int]$y,[IntPtr]$epoch=[IntPtr]::Zero) {
    Require (!$script:injectedLeftDown -and ![SnipReleaseProbe]::LeftIsDown()) 'left-button-already-held'
    Assert-InputTarget $root $ownerPid $x $y $epoch
    Require ([SnipReleaseProbe]::SetCursorPos($x,$y)) 'input-cursor-position-failed'
    Require ([SnipReleaseProbe]::CursorAt($x,$y)) 'input-cursor-moved-before-down'
    Require (![SnipReleaseProbe]::LeftIsDown()) 'left-button-became-held'
    Assert-InputTarget $root $ownerPid $x $y $epoch
    if($script:timeNextButtonDown){$script:buttonDownTimestamp=[System.Diagnostics.Stopwatch]::GetTimestamp();$script:buttonClock=[System.Diagnostics.Stopwatch]::StartNew();$script:timeNextButtonDown=$false}
    # Record BEFORE injecting, so interruption after injection still releases.
    $script:injectedLeftDown = $true
    [SnipReleaseProbe]::mouse_event(2,0,0,0,[UIntPtr]::Zero)
}
function Move-InjectedLeft([IntPtr]$root,[uint32]$ownerPid,[int]$x,[int]$y,[IntPtr]$epoch) {
    Require ($script:injectedLeftDown -and [SnipReleaseProbe]::LeftIsDown()) 'injected-drag-no-longer-held'
    Require ([SnipReleaseProbe]::CursorOver($root)) 'drag-current-target-changed'
    Assert-InputTarget $root $ownerPid $x $y $epoch
    Require ([SnipReleaseProbe]::SetCursorPos($x,$y)) 'drag-cursor-position-failed'
    Require ([SnipReleaseProbe]::CursorAt($x,$y)) 'drag-cursor-moved'
    Assert-InputTarget $root $ownerPid $x $y $epoch
}
function Release-InjectedLeft {
    if($script:injectedLeftDown){
        $script:lastInjectedUpTimestamp=[System.Diagnostics.Stopwatch]::GetTimestamp()
        [SnipReleaseProbe]::mouse_event(4,0,0,0,[UIntPtr]::Zero)
        $script:injectedLeftDown = $false
    }
}
function Test-SnipIdentitySame($expected,$current,[switch]$RequireVisible) {
    if($null -eq $expected -or $null -eq $current){return $false}
    if(!$expected.Live -or !$current.Live -or [long]$expected.Hwnd -eq 0 -or
       [long]$expected.Root -ne [long]$expected.Hwnd -or [long]$current.Root -ne [long]$current.Hwnd -or
       [uint32]$expected.Pid -eq 0 -or [uint32]$expected.Thread -eq 0){return $false}
    foreach($field in @('Hwnd','Root','Pid','Thread','Epoch')){if($expected.$field -ne $current.$field){return $false}}
    if($RequireVisible -and (!$expected.Visible -or !$current.Visible)){return $false}
    return $true
}
function Test-SnipOwnedInventorySame([object[]]$expected,[object[]]$current,[uint32]$OwnerPid) {
    if($OwnerPid -eq 0 -or $expected.Count -eq 0 -or $expected.Count -gt 256 -or $expected.Count -ne $current.Count){return $false}
    $map=@{};$seen=@{}
    foreach($row in $expected){
        if($null -eq $row -or $row.Pid -ne $OwnerPid -or !(Test-SnipIdentitySame $row $row) -or $map.ContainsKey([long]$row.Hwnd)){return $false}
        $map[[long]$row.Hwnd]=$row
    }
    foreach($row in $current){
        if($null -eq $row -or $row.Pid -ne $OwnerPid -or $seen.ContainsKey([long]$row.Hwnd) -or !$map.ContainsKey([long]$row.Hwnd)){return $false}
        if(!(Test-SnipIdentitySame $map[[long]$row.Hwnd] $row)){return $false}
        $seen[[long]$row.Hwnd]=$true
    }
    return $true
}
function Test-SnipInventoryWithImeAdditions([object[]]$original,[object[]]$before,[object[]]$after,
    [object[]]$BeforeHelpers,[object[]]$AfterHelpers,$bar,[uint32]$ShellPid) {
    # No removals/replacements/identity changes, and NO additions after UP.
    if(!(Test-SnipOwnedInventorySame $before $after $ShellPid)){return $false}
    foreach($row in $before){
        $same=@($after|Where-Object {$_.Hwnd -eq $row.Hwnd})
        if($same.Count -ne 1 -or $row.Visible -ne $same[0].Visible){return $false}
    }
    $known=@{};foreach($row in $original){$known[[long]$row.Hwnd]=$row}
    $prior=@($before|Where-Object {$known.ContainsKey([long]$_.Hwnd)})
    if(!(Test-SnipOwnedInventorySame $original $prior $ShellPid)){return $false}
    $added=@($before|Where-Object {!$known.ContainsKey([long]$_.Hwnd)})
    if($added.Count -ne $BeforeHelpers.Count -or $added.Count -ne $AfterHelpers.Count){return $false}
    foreach($row in $added){
        $first=@($BeforeHelpers|Where-Object {$_.Hwnd -eq $row.Hwnd})
        $last=@($AfterHelpers|Where-Object {$_.Hwnd -eq $row.Hwnd})
        if($first.Count -ne 1 -or $last.Count -ne 1){return $false}
        foreach($fact in @($first[0],$last[0])){
            if(!$fact.ClassVerified -or $fact.ClassCode -ne 2 -or !$fact.Live -or $fact.Visible -or $fact.Epoch -ne 0 -or
               $fact.Pid -ne $ShellPid -or $fact.Thread -ne $bar.Thread -or $fact.Root -ne $fact.Hwnd -or
               !(Test-SnipIdentitySame $row $fact)){return $false}
        }
    }
    return $true
}
function Test-SnipPostClickBaseline(
    [object[]]$PreOwned,[object[]]$BeforeOwned,[object[]]$AfterOwned,
    $PreFixture,$CurrentFixture,$PreBar,$CurrentBar,$Observed,
    [long]$ProtectedBarHwnd,[long]$ProtectedBarEpoch,[uint32]$ShellPid,[uint32]$FixturePid,
    [object[]]$BeforeHelpers=@(),[object[]]$AfterHelpers=@()
) {
    $result=[pscustomobject]@{Accepted=$false;Classification='inconclusive';Reason='owned-inventory-changed';Observed=$Observed}
    if(!(Test-SnipInventoryWithImeAdditions $PreOwned $BeforeOwned $AfterOwned $BeforeHelpers $AfterHelpers $PreBar $ShellPid)){return $result}
    $result.Reason='protected-bar-identity-changed'
    if($ProtectedBarHwnd -eq 0 -or $ProtectedBarEpoch -eq 0 -or $PreBar.Hwnd -ne $ProtectedBarHwnd -or
       $PreBar.Pid -ne $ShellPid -or $PreBar.Epoch -ne $ProtectedBarEpoch -or
       !(Test-SnipIdentitySame $PreBar $CurrentBar -RequireVisible)){return $result}
    # Equality above proves this exact row persists in BOTH bracket inventories.
    $bar=@($PreOwned | Where-Object {$_.Hwnd -eq $ProtectedBarHwnd})
    if($bar.Count -ne 1 -or !(Test-SnipIdentitySame $PreBar $bar[0])){return $result}
    $result.Reason='fixture-identity-changed'
    if($FixturePid -eq 0 -or $FixturePid -eq $ShellPid -or $PreFixture.Pid -ne $FixturePid -or
       !(Test-SnipIdentitySame $PreFixture $CurrentFixture -RequireVisible)){return $result}
    $result.Reason='foreground-not-approved'
    if(Test-SnipIdentitySame $PreFixture $Observed -RequireVisible){$result.Classification='fixtureB'}
    elseif(Test-SnipIdentitySame $PreBar $Observed -RequireVisible){$result.Classification='protectedbar'}
    else{return $result}
    $result.Accepted=$true;$result.Reason='accepted';return $result
}
function Get-SnipInventoryDelta([object[]]$before,[object[]]$after,
    [ValidateSet('pre-to-beforeUP','beforeUP-to-afterFG')][string]$transition,[long]$ProtectedBarHwnd) {
    $oldMap=@{};$newMap=@{}
    foreach($row in $before){$oldMap[[long]$row.Hwnd]=$row}
    foreach($row in $after){$newMap[[long]$row.Hwnd]=$row}
    $keys=@((@($oldMap.Keys)+@($newMap.Keys))|Sort-Object -Unique)
    foreach($key in $keys){
        $old=$oldMap[$key];$new=$newMap[$key];$mask=0
        if($null -eq $old){$change='added'}elseif($null -eq $new){$change='retired'}else{
            $bit=1;foreach($field in @('Root','Pid','Thread','Epoch','Live','Visible')){
                if($old.$field -ne $new.$field){$mask=$mask -bor $bit};$bit=$bit -shl 1
            }
            if($mask -eq 0){continue};$change=if($mask -eq 32){'visibilitychanged'}else{'identitychanged'}
        }
        $oldRole=if($null -eq $old){'absent'}elseif($old.Hwnd -eq $ProtectedBarHwnd){'protectedbar'}elseif($old.Epoch -ne 0){'registeredother'}else{'unknown'}
        $newRole=if($null -eq $new){'absent'}elseif($new.Hwnd -eq $ProtectedBarHwnd){'protectedbar'}elseif($new.Epoch -ne 0){'registeredother'}else{'unknown'}
        [pscustomobject]@{Transition=$transition;Change=$change;Hwnd=[long]$key;ChangedMask=$mask;Old=$old;New=$new;OldRole=$oldRole;NewRole=$newRole}
    }
}
function Write-SnipBaselineFailure {
    # Failure-only, bounded scalar projection of the already captured snapshots.
    # Mask bits: root1, PID2, thread4, epoch8, live16, visible32. Roles never infer
    # an overlay from size, class, title or an unbound HWND.
    $deltas=@(Get-SnipInventoryDelta $preClickOwned $beforeForegroundInventory 'pre-to-beforeUP' $top.ToInt64();
        Get-SnipInventoryDelta $beforeForegroundInventory $afterForegroundInventory 'beforeUP-to-afterFG' $top.ToInt64())
    $dropped=[Math]::Max(0,$deltas.Count-32)
    "BASELINE_INVENTORIES preCount=$(@($preClickOwned).Count) beforeUpCount=$(@($beforeForegroundInventory).Count) afterCount=$(@($afterForegroundInventory).Count) deltaCount=$($deltas.Count) deltaDropped=$dropped"
    foreach($delta in @($deltas|Select-Object -First 32)){
        $o=$delta.Old;$n=$delta.New
        "BASELINE_DELTA transition=$($delta.Transition) change=$($delta.Change) hwnd=$($delta.Hwnd) changedMask=$($delta.ChangedMask) oldRole=$($delta.OldRole) oldRoot=$([long]$o.Root) oldPid=$([long]$o.Pid) oldThread=$([long]$o.Thread) oldEpoch=$([long]$o.Epoch) oldLive=$([bool]$o.Live) oldVisible=$([bool]$o.Visible) newRole=$($delta.NewRole) newRoot=$([long]$n.Root) newPid=$([long]$n.Pid) newThread=$([long]$n.Thread) newEpoch=$([long]$n.Epoch) newLive=$([bool]$n.Live) newVisible=$([bool]$n.Visible)"
        if($delta.Change -eq 'added' -and $n.Pid -eq $shellPid){
            $metadata=[SnipReleaseProbe]::AddedRootMetadata([IntPtr]$delta.Hwnd,[uint32]$n.Pid,[uint32]$n.Thread,[long]$n.Epoch,$top,$script:protectedButtonEpoch.ToInt64())
            "BASELINE_ROOT_CLASS hwnd=$($delta.Hwnd) [identityValid,classCode,style,exstyle,ownerHwnd,thread,sameThreadAsBar,epoch,barThread]=$($metadata -join ',')"
        }
    }
    $frequency=[System.Diagnostics.Stopwatch]::Frequency
    $relative=@{};foreach($key in $baselineTicks.Keys){$relative[$key]=[long][Math]::Round(1000000.0*($baselineTicks[$key]-$script:buttonDownTimestamp)/$frequency)}
    $upUs=[long][Math]::Round(1000000.0*($script:lastInjectedUpTimestamp-$script:buttonDownTimestamp)/$frequency)
    "BASELINE_TIMING preInventoryStartUs=$($relative.PreStart) preInventoryEndUs=$($relative.PreEnd) beforeUpInventoryStartUs=$($relative.BeforeStart) beforeUpInventoryEndUs=$($relative.BeforeEnd) upInjectionStartUs=$upUs foregroundReadEndUs=$($relative.ForegroundEnd) afterInventoryStartUs=$($relative.AfterStart) afterInventoryEndUs=$($relative.AfterEnd) decisionStartUs=$($relative.DecisionStart) decisionEndUs=$($relative.DecisionEnd)"
    "BASELINE_DURATION preInventoryUs=$($relative.PreEnd-$relative.PreStart) beforeUpInventoryUs=$($relative.BeforeEnd-$relative.BeforeStart) afterInventoryUs=$($relative.AfterEnd-$relative.AfterStart) decisionUs=$($relative.DecisionEnd-$relative.DecisionStart) diagnosticOverheadUnmeasured=true"
}
function Write-SnipForegroundFailure([IntPtr]$observed,[string]$gate) {
    # Fixed classifications + numeric owned identities only. The observed HWND
    # is the SAME sample used by the failing assertion, not a replacement read.
    $root=if($observed -eq [IntPtr]::Zero){[IntPtr]::Zero}else{[SnipReleaseProbe]::GetAncestor($observed,2)}
    $owner=if($root -eq [IntPtr]::Zero){0}else{[SnipReleaseProbe]::ProcessId($root)}
    $kind=if($observed -eq [IntPtr]::Zero -or $root -eq [IntPtr]::Zero){'none'}
        elseif($root -eq $fixtureA){'fixtureA'}elseif($root -eq $fixture){'fixtureB'}
        elseif(@($overlays) -contains $root){'ownedoverlay'}elseif($root -eq $top){'ownedbar'}
        elseif($owner -eq $shellPid){'ownedother'}else{'externalother'}
    $beforeDownMs=if($script:fixtureBConfirmedTimestamp -gt 0 -and $script:buttonDownTimestamp -gt 0){
        [Math]::Round(1000.0*($script:buttonDownTimestamp-$script:fixtureBConfirmedTimestamp)/[System.Diagnostics.Stopwatch]::Frequency,3)
    }else{-1}
    "FOREGROUND_FAILURE gate=$gate observedHwnd=$($observed.ToInt64()) rootHwnd=$($root.ToInt64()) rootPid=$owner classification=$kind sinceButtonDownMs=$($buttonClock.ElapsedMilliseconds) sinceButtonUpMs=$($buttonClock.ElapsedMilliseconds-$buttonUpMs) lastFixtureBConfirmedBeforeButtonDownMs=$beforeDownMs"
    "FOREGROUND_EXPECTED fixtureBHwnd=$($fixture.ToInt64()) pid=$([SnipReleaseProbe]::ProcessId($fixture)) live=$([SnipReleaseProbe]::IsWindow($fixture)) visible=$([SnipReleaseProbe]::IsWindowVisible($fixture)) fixtureAHwnd=$($fixtureA.ToInt64()) ownedFocusEventsA=$([SnipReleaseProbe]::NeutralFocusCount($fixtureA)) ownedFocusEventsB=$([SnipReleaseProbe]::NeutralFocusCount($fixture)) ownedLastFocus=$([SnipReleaseProbe]::NeutralLastFocus())"
    # ownedbar means the exact protected top-bar HWND, not a size/title heuristic.
    "FOREGROUND_BUTTON hwnd=$($top.ToInt64()) pid=$([SnipReleaseProbe]::ProcessId($top)) injectedEpoch=$($script:protectedButtonEpoch.ToInt64()) currentEpoch=$([SnipReleaseProbe]::GetPropW($top,'JasonShell.Snip.Instance').ToInt64())"
    if($null -ne $postClickBaseline -and $postClickBaseline.Accepted){"FOREGROUND_BASELINE hwnd=$($postClickBaseline.Observed.Hwnd) pid=$($postClickBaseline.Observed.Pid) epoch=$($postClickBaseline.Observed.Epoch) classification=$($postClickBaseline.Classification)"}
    foreach($h in @($overlays)){
        "FOREGROUND_OVERLAY hwnd=$($h.ToInt64()) pid=$([SnipReleaseProbe]::ProcessId($h)) epoch=$([SnipReleaseProbe]::GetPropW($h,'JasonShell.Snip.Instance').ToInt64()) live=$([SnipReleaseProbe]::IsWindow($h)) visible=$([SnipReleaseProbe]::IsWindowVisible($h))"
    }
}
$script:injectedLeftDown = $false
$script:timeNextButtonDown = $false
$script:buttonClock = $null
$script:buttonDownTimestamp = 0L
$script:fixtureBConfirmedTimestamp = 0L
$script:protectedButtonEpoch = [IntPtr]::Zero
$script:lastInjectedUpTimestamp = 0L
$postClickBaseline=$null
$baselineReason='baseline-capture-error'
$stage='preflight'; $shell=$null; $fixture=[IntPtr]::Zero; $top=[IntPtr]::Zero; $shellPid=0; $pass=$false
$oldDpi=[SnipReleaseProbe]::SetThreadDpiAwarenessContext([IntPtr](-4))
try {
    Require (@(Get-Process -Name 'jason-shell' -ErrorAction SilentlyContinue).Count -eq 0) 'existing-shell-conflict'
    Require (![SnipReleaseProbe]::LeftIsDown()) 'left-button-already-held-at-preflight'
    if($AcceptanceProfile -eq 'SingleDisplay'){$singleProfile=Read-SnipSingleProfile;$initialSingleBounds=$singleProfile.Bounds -join ',';$initialSingleDpi=$singleProfile.Dpi}
    $exe=Join-Path $PSScriptRoot '../src-tauri/target/release/jason-shell.exe'
    $shell=Start-Process -FilePath $exe -PassThru
    $shellPid=$shell.Id
    $stage='top-bar-button'
    $button=Wait-Stage {
        foreach($h in [SnipReleaseProbe]::Windows($shellPid)) { $b=Find-Name $h 'Capture screen region'; if($b){$script:top=$h; return $b} }
    } 'top-bar-button' 60
    $stage='post-bar-external-foreground-transition'
    if($AcceptanceProfile -eq 'SingleDisplay'){
        $singleProfile=Read-SnipSingleProfile
        Require (($singleProfile.Bounds -join ',') -eq $initialSingleBounds -and $singleProfile.Dpi -eq $initialSingleDpi) 'single-profile-topology-changed'
        $a=$singleProfile.FixtureA;$b=$singleProfile.FixtureB
        $fixture=[SnipReleaseProbe]::StartNeutralFixturesAt($a[0],$a[1],$a[2],$a[3],$b[0],$b[1],$b[2],$b[3])
        $ax=$singleProfile.ClickA[0];$ay=$singleProfile.ClickA[1];$bx=$singleProfile.ClickB[0];$by=$singleProfile.ClickB[1]
    }else{$fixture=[SnipReleaseProbe]::StartNeutralFixtures();$ax=2650;$ay=350;$bx=2950;$by=350}
    $fixtureA=[SnipReleaseProbe]::NeutralA
    # If creation left A foreground, first switch to B so the next A click is a
    # genuine transition. None of these equalities are shell observer readiness.
    if([SnipReleaseProbe]::GetForegroundWindow() -eq $fixtureA){
        $beforeB=[SnipReleaseProbe]::NeutralFocusCount($fixture)
        Start-InjectedLeft $fixture $PID $bx $by;Release-InjectedLeft
        [void](Wait-Stage { [SnipReleaseProbe]::GetForegroundWindow() -eq $fixture -and [SnipReleaseProbe]::NeutralFocusCount($fixture) -gt $beforeB } 'neutral-initial-B-os-event' 5)
    }
    Require ([SnipReleaseProbe]::GetForegroundWindow() -ne $fixtureA) 'neutral-A-transition-not-distinct'
    $beforeA=[SnipReleaseProbe]::NeutralFocusCount($fixtureA)
    Start-InjectedLeft $fixtureA $PID $ax $ay
    Release-InjectedLeft
    [void](Wait-Stage { [SnipReleaseProbe]::GetForegroundWindow() -eq $fixtureA -and [SnipReleaseProbe]::NeutralFocusCount($fixtureA) -gt $beforeA -and [SnipReleaseProbe]::NeutralLastFocus() -eq 1 } 'neutral-A-os-foreground-event' 5)
    $beforeB=[SnipReleaseProbe]::NeutralFocusCount($fixture)
    Start-InjectedLeft $fixture $PID $bx $by
    Release-InjectedLeft
    [void](Wait-Stage { [SnipReleaseProbe]::GetForegroundWindow() -eq $fixture -and [SnipReleaseProbe]::NeutralFocusCount($fixture) -gt $beforeB -and [SnipReleaseProbe]::NeutralLastFocus() -eq 2 } 'neutral-B-os-foreground-event' 5)
    $script:fixtureBConfirmedTimestamp=[System.Diagnostics.Stopwatch]::GetTimestamp()
    'STAGE post-bar-responsive-external-fixtures genuine-A-to-B=true owned-OS-foreground-events=true shellObserverAcceptance=not-yet-observed'
    $sequenceBefore=[SnipReleaseProbe]::GetClipboardSequenceNumber()
    if($AcceptanceProfile -eq 'SingleDisplay'){
        $singleProfile=Read-SnipSingleProfile
        Require (($singleProfile.Bounds -join ',') -eq $initialSingleBounds -and $singleProfile.Dpi -eq $initialSingleDpi) 'single-profile-topology-changed'
        "PROFILE acceptance=SingleDisplay monitor=$($singleProfile.Bounds -join ',') work=$($singleProfile.WorkArea -join ',') dpi=$($singleProfile.Dpi) expectedSelection=$($singleProfile.SelectionSize)x$($singleProfile.SelectionSize) expectedPreview=$($singleProfile.Preview -join ',') DOMobserved=false"
    }
    $beforeWindows=@([SnipReleaseProbe]::Windows($shellPid))
    $buttonRect=$button.Current.BoundingRectangle
    $clickX=[int]($buttonRect.Left+$buttonRect.Width/2);$clickY=[int]($buttonRect.Top+$buttonRect.Height/2)
    Require ([SnipReleaseProbe]::GetAncestor([SnipReleaseProbe]::At($clickX,$clickY),2) -eq $top) 'snip-button-native-hit-test-failed'
    Require $button.Current.IsEnabled 'snip-button-disabled-before-click'
    $script:timeNextButtonDown=$true
    $script:protectedButtonEpoch=[SnipReleaseProbe]::GetPropW($top,'JasonShell.Snip.Instance')
    if($AcceptanceProfile -eq 'SingleDisplay'){
        $stage='post-click-foreground-baseline'
        $baselineTicks=@{PreStart=[System.Diagnostics.Stopwatch]::GetTimestamp()}
        $preClickOwned=[SnipReleaseProbe]::AllOwnedIdentities($shellPid)
        $baselineTicks.PreEnd=[System.Diagnostics.Stopwatch]::GetTimestamp()
        $preClickFixture=[SnipReleaseProbe]::Identity($fixture)
        $preClickBar=[SnipReleaseProbe]::Identity($top)
        if($script:protectedButtonEpoch -eq [IntPtr]::Zero -or $preClickBar.Epoch -ne $script:protectedButtonEpoch.ToInt64() -or
           $preClickBar.Pid -ne $shellPid -or !(Test-SnipIdentitySame $preClickBar $preClickBar -RequireVisible)){
            $baselineReason='protected-bar-identity-changed';throw 'post-click-baseline-inconclusive'
        }
    }
    Start-InjectedLeft $top $shellPid $clickX $clickY $script:protectedButtonEpoch
    Start-Sleep -Milliseconds 100
    if($AcceptanceProfile -eq 'SingleDisplay'){
        # The BEFORE inventory is taken while the injected button is still held.
        # UP -> foreground read is immediate: no log, pump, sleep, wait or inventory
        # between them. The AFTER inventory includes hidden/unbound owned roots.
        # Enumerations are snapshots, not an atomic creation-event ledger: a root
        # born and retired entirely between snapshots cannot be ruled out. Never
        # retry/adopt a post-show sample to conceal an observed inventory change.
        $baselineTicks.BeforeStart=[System.Diagnostics.Stopwatch]::GetTimestamp()
        $beforeForegroundInventory=[SnipReleaseProbe]::AllOwnedIdentities($shellPid)
        $baselineTicks.BeforeEnd=[System.Diagnostics.Stopwatch]::GetTimestamp()
        $beforeHelperFacts=[SnipReleaseProbe]::VerifyAddedRootClasses($preClickOwned,$beforeForegroundInventory,$top,$script:protectedButtonEpoch.ToInt64())
        Release-InjectedLeft
        $postClickForeground=[SnipReleaseProbe]::GetForegroundWindow()
        $baselineTicks.ForegroundEnd=[System.Diagnostics.Stopwatch]::GetTimestamp()
        $postClickObserved=[SnipReleaseProbe]::Identity($postClickForeground)
        $baselineTicks.AfterStart=[System.Diagnostics.Stopwatch]::GetTimestamp()
        $afterForegroundInventory=[SnipReleaseProbe]::AllOwnedIdentities($shellPid)
        $baselineTicks.AfterEnd=[System.Diagnostics.Stopwatch]::GetTimestamp()
        $afterHelperFacts=[SnipReleaseProbe]::VerifyAddedRootClasses($preClickOwned,$afterForegroundInventory,$top,$script:protectedButtonEpoch.ToInt64())
        $postClickFixture=[SnipReleaseProbe]::Identity($fixture)
        $postClickBar=[SnipReleaseProbe]::Identity($top)
        $baselineTicks.DecisionStart=[System.Diagnostics.Stopwatch]::GetTimestamp()
        $postClickBaseline=Test-SnipPostClickBaseline -PreOwned $preClickOwned -BeforeOwned $beforeForegroundInventory -AfterOwned $afterForegroundInventory -PreFixture $preClickFixture -CurrentFixture $postClickFixture -PreBar $preClickBar -CurrentBar $postClickBar -Observed $postClickObserved -ProtectedBarHwnd $top.ToInt64() -ProtectedBarEpoch $script:protectedButtonEpoch.ToInt64() -ShellPid $shellPid -FixturePid $PID -BeforeHelpers $beforeHelperFacts -AfterHelpers $afterHelperFacts
        $baselineTicks.DecisionEnd=[System.Diagnostics.Stopwatch]::GetTimestamp()
        $baselineReason=$postClickBaseline.Reason
        if(!$postClickBaseline.Accepted){Write-SnipBaselineFailure;throw 'post-click-baseline-inconclusive'}
    }else{Release-InjectedLeft}
    $buttonUpMs=if($AcceptanceProfile -eq 'SingleDisplay'){
        [long][Math]::Floor(1000.0*($script:lastInjectedUpTimestamp-$script:buttonDownTimestamp)/[System.Diagnostics.Stopwatch]::Frequency)
    }else{$buttonClock.ElapsedMilliseconds}
    "STAGE actual-release-top-bar-button-native-click point=$clickX,$clickY"
    "TIMING monotonic buttonDownT0=true buttonUpMs=$buttonUpMs immediateFeedbackSampled=false"
    if($AcceptanceProfile -eq 'SingleDisplay'){
        "FOREGROUND_BASELINE accepted=true classification=$($postClickBaseline.Classification) hwnd=$($postClickBaseline.Observed.Hwnd) pid=$($postClickBaseline.Observed.Pid) epoch=$($postClickBaseline.Observed.Epoch) preClickIdentitiesUnchanged=true beforeAfterInventoriesUnchanged=true hiddenUnboundRootsIncluded=true verifiedImeAdditions=$(@($beforeHelperFacts).Count)"
        foreach($helper in @($beforeHelperFacts)){"FOREGROUND_ACCEPTED_HELPER hwnd=$($helper.Hwnd) root=$($helper.Root) pid=$($helper.Pid) thread=$($helper.Thread) epoch=$($helper.Epoch) live=$($helper.Live) visible=$($helper.Visible) classCode=$($helper.ClassCode) verifiedBeforeAndAfter=true"}
    }
    $stage=if($AcceptanceProfile -eq 'SingleDisplay'){'single-overlay-inferred-visible'}else{'both-overlays-inferred-visible'}
    $maxRegistered=0
    $visibilityClock=$buttonClock
    $observedInstances=@{}
    $visibilityTraceCount=0
    $firstNewVisibleMs=$null
    $overlays=Wait-Stage {
        $registered=@([SnipReleaseProbe]::Registered($shellPid))
        $count=$registered.Count;if($count -gt $script:maxRegistered){$script:maxRegistered=$count}
        # Read-only bounded geometry metadata. No desktop pixels, titles, IPC
        # commands or arming claims. Transiently destroyed handles are ignored.
        $present=@{}
        foreach($handle in $registered){
            try {
                $epoch=[SnipReleaseProbe]::GetPropW($handle,'JasonShell.Snip.Instance').ToInt64()
                $r=[SnipReleaseProbe]::Bounds($handle)
                $visible=[SnipReleaseProbe]::IsWindowVisible($handle)
                if($visible -and ($beforeWindows -notcontains $handle) -and $null -eq $script:firstNewVisibleMs){
                    $script:firstNewVisibleMs=$buttonClock.ElapsedMilliseconds
                    "TIMING firstNewRegisteredVisibleMs=$script:firstNewVisibleMs interactiveNotEstablished=true"|Out-Host
                }
                $present[$epoch]=$true
                $state="$visible|$($r -join ',')"
                if($script:observedInstances[$epoch] -ne $state -and $script:visibilityTraceCount -lt 24){
                    "TRACE instance=$epoch elapsedMs=$($visibilityClock.ElapsedMilliseconds) visible=$visible bounds=$($r -join ',')" | Out-Host
                    $script:visibilityTraceCount++
                }
                $script:observedInstances[$epoch]=$state
            } catch { }
        }
        foreach($epoch in @($script:observedInstances.Keys)){
            if(!$present.ContainsKey($epoch) -and $script:observedInstances[$epoch] -ne 'retired'){
                if($script:visibilityTraceCount -lt 24){"TRACE instance=$epoch elapsedMs=$($visibilityClock.ElapsedMilliseconds) noLongerRegistered=true" | Out-Host;$script:visibilityTraceCount++}
                $script:observedInstances[$epoch]='retired'
            }
        }
        if($AcceptanceProfile -eq 'SingleDisplay'){
            $found=@([SnipReleaseProbe]::Windows($shellPid)|Where-Object {($beforeWindows -notcontains $_) -and [SnipReleaseProbe]::GetPropW($_,'JasonShell.Snip.Instance') -ne [IntPtr]::Zero})
            $newRegistered=@([SnipReleaseProbe]::Registered($shellPid)|Where-Object {$beforeWindows -notcontains $_})
            if($newRegistered.Count -eq 1 -and $found.Count -eq 1 -and $newRegistered[0] -eq $found[0] -and ([SnipReleaseProbe]::Bounds($found[0]) -join ',') -eq ($singleProfile.Bounds -join ',')){return ,$found}
            return $null
        }
        $found=@(); foreach($h in [SnipReleaseProbe]::Windows($shellPid)) {
            if(($beforeWindows -notcontains $h) -and [SnipReleaseProbe]::GetPropW($h,'JasonShell.Snip.Instance') -ne [IntPtr]::Zero) {
                $r=[SnipReleaseProbe]::Bounds($h)
                if(($r[2] -eq 2560 -and $r[3] -eq 1440) -or ($r[2] -eq 3840 -and $r[3] -eq 2160)){$found += $h}
            }
        }
        if($found.Count -eq 2){return ,$found}
    } $stage 30
    $rectangles=@(); foreach($h in $overlays){$r=[SnipReleaseProbe]::Bounds($h);$rectangles += ,$r;[uint32]$affinity=0;Require ([SnipReleaseProbe]::GetWindowDisplayAffinity($h,[ref]$affinity) -and $affinity -eq 17) 'overlay-exclusion-failed'}
    if($AcceptanceProfile -eq 'SingleDisplay'){
        Require ($overlays.Count -eq 1 -and ($rectangles[0] -join ',') -eq ($singleProfile.Bounds -join ',')) 'single-overlay-bounds-failed'
        $singleClient=[SnipReleaseProbe]::ClientBounds($overlays[0])
        $singleWindowDpi=[SnipReleaseProbe]::GetDpiForWindow($overlays[0])
        "PROFILE sameHWND-client outer=$($rectangles[0] -join ',') client=$($singleClient -join ',') expected=$($singleProfile.Bounds -join ',') windowDpi=$singleWindowDpi expectedDpi=$($singleProfile.Dpi) pmv2=$([SnipReleaseProbe]::AreDpiAwarenessContextsEqual([SnipReleaseProbe]::GetThreadDpiAwarenessContext(),[IntPtr](-4)))"
        if(($singleClient -join ',') -ne ($singleProfile.Bounds -join ',')){
            # Failure-only numeric metadata; never mutates geometry or acceptance.
            # Resolve the same owned HWND/epoch before observing style/DWM frame.
            $failedEpoch=[SnipReleaseProbe]::GetPropW($overlays[0],'JasonShell.Snip.Instance')
            if([SnipReleaseProbe]::ProcessId($overlays[0]) -eq $shellPid -and $failedEpoch -ne [IntPtr]::Zero){
                "DIAG clientMismatch hwnd=$($overlays[0].ToInt64()) epoch=$($failedEpoch.ToInt64()) [style,exstyle,dwmHresult,dwmL,T,W,H]=$([SnipReleaseProbe]::FailureFrameMetadata($overlays[0]) -join ',')"
            }
        }
        Require (($singleClient -join ',') -eq ($singleProfile.Bounds -join ',')) 'single-overlay-client-coverage-failed'
        Require ($singleWindowDpi -eq $singleProfile.Dpi) 'single-overlay-dpi-failed'
        $overlayForeground=[SnipReleaseProbe]::GetForegroundWindow()
        $overlayForegroundIdentity=[SnipReleaseProbe]::Identity($overlayForeground)
        $sameBaseline=Test-SnipIdentitySame $postClickBaseline.Observed $overlayForegroundIdentity -RequireVisible
        if(!$sameBaseline){Write-SnipForegroundFailure $overlayForeground 'single-overlay'}
        Require $sameBaseline 'single-overlay-changed-postclick-foreground'
        "STAGE single-overlay-inferred-visible exact=$($singleProfile.Bounds -join ',') fullClient=true excluded=true foregroundUnchanged=true directAckOrArmingObserved=false"
        "TIMING singleExactVisibleValidatedMs=$($buttonClock.ElapsedMilliseconds) directAckOrArmingObserved=false"
    }else{
    Require (@($rectangles | Where-Object {($_ -join ',') -eq '0,0,2560,1440'}).Count -eq 1) 'primary-overlay-bounds-failed'
    Require (@($rectangles | Where-Object {($_ -join ',') -eq '2560,0,3840,2160'}).Count -eq 1) 'secondary-overlay-bounds-failed'
    'STAGE both-overlays-inferred-visible exact=(0,0,2560,1440)|(2560,0,3840,2160) excluded=true directAckOrArmingObserved=false'
    "TIMING bothExactVisibleValidatedMs=$($buttonClock.ElapsedMilliseconds) directAckOrArmingObserved=false"
    }
    $stage='secondary-pointer-selection'
    if($AcceptanceProfile -eq 'SingleDisplay'){$secondary=@($overlays)}else{$secondary=@($overlays | Where-Object {([SnipReleaseProbe]::Bounds($_) -join ',') -eq '2560,0,3840,2160'})}
    Require ($secondary.Count -eq 1) 'secondary-input-target-ambiguous'
    $secondaryEpoch=[SnipReleaseProbe]::GetPropW($secondary[0],'JasonShell.Snip.Instance')
    Require ($secondaryEpoch -ne [IntPtr]::Zero) 'secondary-input-target-unregistered'
    # Provenance: tests/snipping_png_codec_baseline.md, prior numeric-only CDP;
    # observed DPR3/DOM origin0,0; native metadata client screen origin2578,0.
    # These physical endpoints predict integral CSS141,134 ->221,194 (80x60),
    # avoiding fractional-edge expansion. Clean runs do not observe DOM coordinates.
    if($AcceptanceProfile -eq 'SingleDisplay'){
        Require (([SnipReleaseProbe]::Bounds($secondary[0]) -join ',') -eq ($singleProfile.Bounds -join ',') -and ([SnipReleaseProbe]::ClientBounds($secondary[0]) -join ',') -eq ($singleProfile.Bounds -join ',')) 'single-overlay-coverage-changed-before-input'
        $sx=$singleProfile.SelectionStart[0];$sy=$singleProfile.SelectionStart[1];$size=$singleProfile.SelectionSize
        # Predicted CSS96,96 ->192,192 with exact client origin and scale=dpi/96.
        # No DOM sample is claimed; exact clipboard dimensions remain runtime oracle.
        Start-InjectedLeft $secondary[0] $shellPid $sx $sy $secondaryEpoch
    }else{Start-InjectedLeft $secondary[0] $shellPid 3001 402 $secondaryEpoch}
    "TIMING selectionDownInjectedMs=$($buttonClock.ElapsedMilliseconds) acceptanceNotYetEstablished=true"
    if($AcceptanceProfile -eq 'SingleDisplay'){
        for($step=1;$step -le 12;$step++){
            Require (([SnipReleaseProbe]::Bounds($secondary[0]) -join ',') -eq ($singleProfile.Bounds -join ',') -and ([SnipReleaseProbe]::ClientBounds($secondary[0]) -join ',') -eq ($singleProfile.Bounds -join ',')) 'single-overlay-coverage-changed-during-input'
            $offset=[int][Math]::Round($size*$step/12.0,[MidpointRounding]::AwayFromZero)
            Move-InjectedLeft $secondary[0] $shellPid ($sx+$offset) ($sy+$offset) $secondaryEpoch;[SnipReleaseProbe]::Pump();Start-Sleep -Milliseconds 25
        }
    }else{for($step=1;$step -le 12;$step++){Move-InjectedLeft $secondary[0] $shellPid (3001+20*$step) (402+15*$step) $secondaryEpoch;[SnipReleaseProbe]::Pump();Start-Sleep -Milliseconds 25}}
    Release-InjectedLeft
    "TIMING selectionUpInjectedMs=$($buttonClock.ElapsedMilliseconds)"
    if($AcceptanceProfile -eq 'SingleDisplay'){"STAGE actual-native-pointer-selection physical=$($size)x$size single-monitor=true predictedCSS=96,96-to-192,192 DOMobserved=false"}else{'STAGE actual-native-pointer-selection physical=240x180 secondary-monitor=true guardedStart=3001,402 guardedEnd=3241,582'}
    $stage='preview-visible'
    $preview=Wait-Stage {
        foreach($h in [SnipReleaseProbe]::Windows($shellPid)){if((Find-Name $h 'Dismiss') -and (Find-Name $h 'Snip preview')){return $h}}
    } 'preview-visible' 25
    "TIMING previewFoundMs=$($buttonClock.ElapsedMilliseconds)"
    $bounds=[SnipReleaseProbe]::Bounds($preview)
    if($AcceptanceProfile -eq 'SingleDisplay'){
        $nowProfile=Read-SnipSingleProfile
        Require (($nowProfile.Bounds -join ',') -eq ($singleProfile.Bounds -join ',') -and ($nowProfile.WorkArea -join ',') -eq ($singleProfile.WorkArea -join ',') -and $nowProfile.Dpi -eq $singleProfile.Dpi) 'single-profile-topology-workarea-changed'
        Require (($bounds -join ',') -eq ($singleProfile.Preview -join ',')) 'preview-exact-single-bounds-failed'
    }else{Require (($bounds -join ',') -eq '5440,0,960,792') 'preview-exact-secondary-bounds-failed'}
    $foreground=[SnipReleaseProbe]::GetForegroundWindow()
    if($foreground -eq $preview -or $foreground -ne $fixture){Write-SnipForegroundFailure $foreground 'preview'}
    Require ($foreground -ne $preview) 'preview-activated'
    Require ($foreground -eq $fixture) 'external-foreground-not-restored'
    [uint32]$previewAffinity=0
    Require ([SnipReleaseProbe]::GetWindowDisplayAffinity($preview,[ref]$previewAffinity) -and $previewAffinity -eq 17) 'preview-exclusion-failed'
    if($AcceptanceProfile -eq 'SingleDisplay'){"STAGE preview-visible exact=$($singleProfile.Preview -join ',') single-monitor=true excluded=true nonactivating=true external-foreground-restored=true"}else{'STAGE preview-visible exact=(5440,0,960,792) on-secondary=true excluded=true nonactivating=true external-foreground-restored=true'}
    "TIMING previewValidatedMs=$($buttonClock.ElapsedMilliseconds) selectionCompletionObserved=true"
    $stage='clipboard-image-metadata'
    "DIAG stage=clipboard-before-header elapsedMs=$($visibilityClock.ElapsedMilliseconds) sequenceBefore=$sequenceBefore sequenceNow=$([SnipReleaseProbe]::GetClipboardSequenceNumber())"
    "DIAG clipboard-allowlisted-header $([SnipReleaseProbe]::ClipboardMetadata())"
    $header=Wait-Stage {[SnipReleaseProbe]::ClipboardHeader()} 'clipboard-image-metadata' 10
    "DIAG stage=clipboard-parsed-header format=$($header[0]) width=$($header[1]) height=$($header[2]) bpp=$($header[3]) sequenceBefore=$sequenceBefore sequenceAfter=$([SnipReleaseProbe]::GetClipboardSequenceNumber()) elapsedMs=$($visibilityClock.ElapsedMilliseconds)"
    if($AcceptanceProfile -eq 'SingleDisplay'){Require ($header[1] -eq $singleProfile.SelectionSize -and $header[2] -eq $singleProfile.SelectionSize) 'clipboard-single-selection-dimensions-failed'}else{Require ($header[1] -eq 240 -and $header[2] -eq 180) 'clipboard-selection-dimensions-failed'}
    Require ([SnipReleaseProbe]::GetClipboardSequenceNumber() -ne $sequenceBefore) 'clipboard-sequence-unchanged'
    "STAGE clipboard-image format=$($header[0]) dimensions=$($header[1])x$($header[2]) bitDepth=$($header[3]) sequenceChanged=true pixelsNotRead=true"
    $stage='dismiss'
    Invoke-Element (Find-Name $preview 'Dismiss')
    [void](Wait-Stage { ![SnipReleaseProbe]::IsWindow($preview) -and @($overlays | Where-Object {[SnipReleaseProbe]::IsWindow($_)}).Count -eq 0 } 'snip-windows-dismissed')
    if($AcceptanceProfile -eq 'SingleDisplay'){'STAGE dismiss preview-and-single-overlay-gone=true'}else{'STAGE dismiss preview-and-both-overlays-gone=true'}
    "TIMING dismissConfirmedMs=$($buttonClock.ElapsedMilliseconds)"
    $pass=$true
} catch {
    # Inventory/verification exceptions can occur while the recorded click is
    # still DOWN. Release that recorded press BEFORE diagnostics/owned cleanup.
    if($stage -eq 'post-click-foreground-baseline'){Release-InjectedLeft}
    # Durable version does not emit arbitrary exception text (may contain private
    # OS/provider diagnostics). Stage + fixed bounded counters define the failure.
    if($stage -eq 'post-click-foreground-baseline'){
        "RESULT INCONCLUSIVE stage=$stage reason=$baselineReason productActivationFailureEstablished=false"
    }else{"RESULT FAIL stage=$stage exceptionType=$($_.Exception.GetType().Name)"}
    if($_.Exception.Message -ceq "timeout:$stage"){'DIAG failureCode=stage-timeout'}
    if($_.Exception.Message -cin @('single-overlay-bounds-failed','single-overlay-client-coverage-failed','single-overlay-dpi-failed',
        'single-overlay-changed-postclick-foreground','single-overlay-coverage-changed-before-input',
        'single-overlay-coverage-changed-during-input','single-profile-topology-changed',
        'single-profile-topology-workarea-changed','preview-exact-single-bounds-failed',
        'clipboard-single-selection-dimensions-failed')){"DIAG failureCode=$($_.Exception.Message)"}
    if($stage -in @('both-overlays-inferred-visible','single-overlay-inferred-visible')){"DIAG maxRegisteredInstances=$maxRegistered buttonEnabled=$($button.Current.IsEnabled)"}
    if($shellPid -gt 0){foreach($h in [SnipReleaseProbe]::Windows($shellPid)){"DIAG owned-visible-window bounds=$([SnipReleaseProbe]::Bounds($h) -join ',') registered=$([SnipReleaseProbe]::GetPropW($h,'JasonShell.Snip.Instance') -ne [IntPtr]::Zero)"}}
    if($top -ne [IntPtr]::Zero){foreach($known in @('Could not capture screen. Try again.','Screen snip timed out. Try again.')){if(Find-Name $top $known){"DIAG known-alert=$known"}}}
} finally {
    # Must precede window closure even after failure/interruption mid-click/drag.
    Release-InjectedLeft
    # Never kill by name. On normal-close timeout report BLOCKED, leave exact PID
    # available for owner-controlled cleanup, and do not launch another instance.
    if($shell -and !$shell.HasExited){
        if($top -ne [IntPtr]::Zero){[void][SnipReleaseProbe]::PostMessageW($top,16,[IntPtr]::Zero,[IntPtr]::Zero)}else{[void]$shell.CloseMainWindow()}
        if(!$shell.WaitForExit(60000)){"CLEANUP BLOCKED owned-shell-normal-close-timeout pid=$shellPid";$pass=$false}else{'CLEANUP owned-release-normal-exit=true'}
    }
    if([SnipReleaseProbe]::StopNeutralFixtures()){'CLEANUP owned-neutral-fixtures-normal-exit=true recorder-unhooked=true'}else{'CLEANUP BLOCKED owned-neutral-fixtures-close-timeout';$pass=$false}
    if($oldDpi -ne [IntPtr]::Zero){[void][SnipReleaseProbe]::SetThreadDpiAwarenessContext($oldDpi)}
}
if($pass){'RESULT PASS actual-release-native-journey';exit 0}else{exit 1}
