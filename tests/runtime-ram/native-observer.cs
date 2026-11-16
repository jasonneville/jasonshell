using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class RamNative {
 public delegate bool EnumProc(IntPtr h, IntPtr l);
 [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc cb, IntPtr l);
 [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint p);
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder b, int n);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
 public class Window { public long Handle; public string Title; public bool Visible; }
 public static Window[] Windows(int pid) {
  var list=new List<Window>();
  EnumWindows((h,l)=> { uint p; GetWindowThreadProcessId(h,out p); if(p==pid) { var s=new StringBuilder(512); GetWindowText(h,s,512); list.Add(new Window {Handle=h.ToInt64(),Title=s.ToString(),Visible=IsWindowVisible(h)}); } return true; },IntPtr.Zero);
  return list.ToArray();
 }
}
