using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
public static class DesktopInspector {
 [StructLayout(LayoutKind.Sequential)] public struct Rect {public int Left,Top,Right,Bottom;}
 [StructLayout(LayoutKind.Sequential)] public struct Point {public int X,Y;}
 [StructLayout(LayoutKind.Sequential)] public struct AppBar {public uint Size;public IntPtr Window;public uint Callback,Edge;public Rect Rect;public IntPtr Param;}
 [DllImport("user32.dll",SetLastError=true)] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
 [DllImport("user32.dll",SetLastError=true)] static extern bool SystemParametersInfo(uint a,uint b,out Rect r,uint f);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr FindWindow(string c,string t);
 [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] static extern bool IsWindow(IntPtr h);
 [DllImport("user32.dll",SetLastError=true)] static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll",SetLastError=true)] static extern bool GetCursorPos(out Point p);
 [DllImport("user32.dll",EntryPoint="GetWindowLongPtrW",SetLastError=true)] static extern IntPtr GetWindowLongPtr(IntPtr h,int index);
 [DllImport("user32.dll",EntryPoint="GetWindowLongW",SetLastError=true)] static extern int GetWindowLong(IntPtr h,int index);
 [DllImport("kernel32.dll")] static extern void SetLastError(uint error);
 [DllImport("shell32.dll")] static extern UIntPtr SHAppBarMessage(uint m,ref AppBar d);
 public class State {public Rect WorkArea,TaskbarRect;public Point Cursor;public uint ForegroundPid;public bool Visible;public long Handle,ExplorerCreatedTicks;public int ExplorerPid;public ulong AppbarState,Style,ExStyle;public string DpiContext;public DateTime Utc;}
 static ulong StyleBits(IntPtr h,int index) {
  SetLastError(0);
  long bits=IntPtr.Size==8?GetWindowLongPtr(h,index).ToInt64():GetWindowLong(h,index);
  if(bits==0 && Marshal.GetLastWin32Error()!=0)throw new Exception("Window style read failed");
  return unchecked((ulong)bits)&0xffffffffUL;
 }
 static long Creation(uint pid) {using(var p=Process.GetProcessById((int)pid)){var handle=p.SafeHandle;return p.StartTime.ToUniversalTime().Ticks;}}
 public static State Read() {
  var prior=SetThreadDpiAwarenessContext(new IntPtr(-4));
  if(prior==IntPtr.Zero)throw new Exception("Cannot set PMv2 observer DPI context");
  try {
   Rect work,bar; if(!SystemParametersInfo(48,0,out work,0))throw new Exception("Workarea read failed");
   var h=FindWindow("Shell_TrayWnd",null); if(h==IntPtr.Zero||!GetWindowRect(h,out bar))throw new Exception("Taskbar read failed");
   uint pid;if(GetWindowThreadProcessId(h,out pid)==0||pid==0)throw new Exception("Explorer PID read failed");
   var created=Creation(pid);var data=new AppBar{Size=(uint)Marshal.SizeOf<AppBar>(),Window=h};
   Point cursor;if(!GetCursorPos(out cursor))throw new Exception("Cursor read failed");
   uint foreground;var foregroundWindow=GetForegroundWindow();GetWindowThreadProcessId(foregroundWindow,out foreground);
   var result=new State{WorkArea=work,TaskbarRect=bar,Cursor=cursor,ForegroundPid=foreground,Visible=IsWindowVisible(h),Handle=h.ToInt64(),ExplorerPid=(int)pid,ExplorerCreatedTicks=created,AppbarState=SHAppBarMessage(4,ref data).ToUInt64(),Style=StyleBits(h,-16),ExStyle=StyleBits(h,-20),DpiContext="per-monitor-v2; physical pixel coordinates",Utc=DateTime.UtcNow};
   uint endPid;Rect endWork,endBar;Point endCursor;
   if(!IsWindow(h)||FindWindow("Shell_TrayWnd",null)!=h||GetWindowThreadProcessId(h,out endPid)==0||endPid!=pid||Creation(endPid)!=created)throw new Exception("Mixed taskbar/Explorer identity during observation");
   if(!GetWindowRect(h,out endBar)||!SystemParametersInfo(48,0,out endWork,0)||!GetCursorPos(out endCursor))throw new Exception("End-of-read state validation failed");
   if(!bar.Equals(endBar)||!work.Equals(endWork)||!cursor.Equals(endCursor)||result.Visible!=IsWindowVisible(h)||result.Style!=StyleBits(h,-16)||result.ExStyle!=StyleBits(h,-20)||result.AppbarState!=SHAppBarMessage(4,ref data).ToUInt64())throw new Exception("Mixed desktop state during observation");
   return result;
  } finally {
   if(SetThreadDpiAwarenessContext(prior)==IntPtr.Zero)throw new Exception("Observer DPI context restoration failed");
  }
 }
}
