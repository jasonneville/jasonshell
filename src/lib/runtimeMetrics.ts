import { invoke } from '@tauri-apps/api/core';

type FrontendSurfaceMetrics = {
  label: 'top-bar' | 'bottom-bar';
  outerHeight: number;
  innerHeight: number;
  clientHeight: number;
};

export type ShellSurfaceRuntimeMetrics = {
  label: string;
  nativeRect: {
    left: number;
    top: number;
    right: number;
    bottom: number;
    width: number;
    height: number;
  };
  outerHeight: number;
  innerHeight: number;
  clientHeight: number;
  nativeHeightOk: boolean;
  webviewHeightOk: boolean;
};

export async function reportShellSurfaceRuntimeMetrics(
  label: FrontendSurfaceMetrics['label']
): Promise<ShellSurfaceRuntimeMetrics> {
  return invoke('report_shell_surface_runtime_metrics', {
    metrics: {
      label,
      outerHeight: Math.round(window.outerHeight),
      innerHeight: Math.round(window.innerHeight),
      clientHeight: Math.round(document.documentElement.clientHeight)
    }
  });
}
