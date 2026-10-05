// Adapted in-app from Rare UI Fluid Orb by Swami Malode (copyright 2026).
// https://rareui.com/components/fluidorb — full terms: speechIndicatorOrb.LICENSE
// Upstream source: components/ui/fluid-orb.tsx, ff043c32397895fa3480af59c1fbec7524481cd1.
const vertexSource = `
attribute vec2 a_pos;
void main() { gl_Position = vec4(a_pos, 0.0, 1.0); }
`;

const fragmentSource = `
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
uniform vec2 u_resolution;
uniform float u_time;
uniform vec3 u_color;
float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}
float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(
    mix(hash(i + vec2(0.0, 0.0)), hash(i + vec2(1.0, 0.0)), u.x),
    mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}
float fbm(vec2 p) {
  float v = 0.0;
  float a = 0.6;
  for (int i = 0; i < 3; i++) {
    v += a * noise(p);
    p *= 2.0;
    a *= 0.5;
  }
  return v;
}
void main() {
  vec2 uv = gl_FragCoord.xy / u_resolution.xy;
  float t = u_time * 0.22;
  vec2 drift = vec2(
    sin(t) + 0.6 * sin(t * 1.7 + 1.3),
    cos(t * 0.8) + 0.6 * cos(t * 1.3 + 2.1));
  vec2 p = vec2(uv.x * 1.8, uv.y * 1.0) + drift * 0.7;
  vec2 q = vec2(fbm(p + drift), fbm(p + vec2(3.2, 1.5) - drift));
  float f = fbm(p + 1.2 * q);
  float g = clamp(1.0 - uv.y, 0.0, 1.0);
  float anchor = smoothstep(0.0, 0.3, uv.y);
  float shade = clamp(g + (f - 0.5) * 0.8 * anchor, 0.0, 1.0);
  vec3 white = vec3(0.99, 1.0, 1.0);
  vec3 light = mix(white, u_color, 0.5);
  vec3 col = mix(white, light, smoothstep(0.28, 0.52, shade));
  col = mix(col, u_color, smoothstep(0.58, 0.88, shade));
  // The host clips the circle. Unlike upstream, every GPU pixel is opaque.
  gl_FragColor = vec4(col, 1.0);
}
`;

// Shell themes currently use hex; computed CSS may serialize to rgb(). Reject
// unresolved/invalid tokens rather than ever passing NaN to a GPU uniform.
function resolvedAccent(): [number, number, number] {
  const token = getComputedStyle(document.documentElement).getPropertyValue('--js-color-accent').trim();
  const hex = /^#([\da-f]{3}|[\da-f]{6})$/i.exec(token);
  if (hex) {
    const value = hex[1].length === 3 ? [...hex[1]].map((digit) => digit + digit).join('') : hex[1];
    return [0, 2, 4].map((offset) => parseInt(value.slice(offset, offset + 2), 16) / 255) as [number, number, number];
  }
  const rgb = /^rgb\(\s*([\d.]+)(%?)\s*[, ]\s*([\d.]+)(%?)\s*[, ]\s*([\d.]+)(%?)\s*\)$/i.exec(token);
  if (rgb) {
    const channels = [1, 3, 5].map((index) => Number(rgb[index]) / (rgb[index + 1] ? 100 : 255));
    if (channels.every((value) => Number.isFinite(value) && value >= 0 && value <= 1)) return channels as [number, number, number];
  }
  // Same opaque fallback as the surface when the root token is unavailable.
  return [192 / 255, 216 / 255, 231 / 255];
}

/** Tiny native-WebGL decoration; no voice level, IPC or window ownership. */
export function createSpeechIndicatorOrb(canvas: HTMLCanvasElement) {
  let gl: WebGLRenderingContext | null = null;
  let program: WebGLProgram | null = null;
  let buffer: WebGLBuffer | null = null;
  const shaders: WebGLShader[] = [];
  let frame = 0;
  let active = false;
  let failed = false;
  let disposed = false;
  let elapsed = 0;
  let previous: number | null = null;
  let time: WebGLUniformLocation | null = null;
  let color: WebGLUniformLocation | null = null;
  let accent = resolvedAccent();
  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)');
  const forced = window.matchMedia('(forced-colors: active)');

  function stop() {
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
    previous = null;
  }
  function release() {
    stop();
    if (gl) {
      if (buffer) gl.deleteBuffer(buffer);
      if (program) gl.deleteProgram(program);
      for (const shader of shaders) gl.deleteShader(shader);
    }
    buffer = null;
    program = null;
    shaders.length = 0;
  }
  function fail() {
    failed = true;
    canvas.style.visibility = 'hidden';
    release();
  }
  function compile(type: number, source: string) {
    const shader = gl!.createShader(type);
    if (!shader) throw new Error('Shader unavailable');
    shaders.push(shader);
    gl!.shaderSource(shader, source);
    gl!.compileShader(shader);
    if (!gl!.getShaderParameter(shader, gl!.COMPILE_STATUS)) throw new Error('Shader failed');
    return shader;
  }
  function initialize() {
    try {
      gl = canvas.getContext('webgl', { alpha: false, antialias: false });
      if (!gl) { fail(); return; }
      program = gl.createProgram();
      if (!program) throw new Error('Program unavailable');
      gl.attachShader(program, compile(gl.VERTEX_SHADER, vertexSource));
      gl.attachShader(program, compile(gl.FRAGMENT_SHADER, fragmentSource));
      gl.linkProgram(program);
      if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error('Link failed');
      gl.useProgram(program);
      buffer = gl.createBuffer();
      if (!buffer) throw new Error('Buffer unavailable');
      gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
      gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]), gl.STATIC_DRAW);
      const position = gl.getAttribLocation(program, 'a_pos');
      if (position < 0) throw new Error('Attribute unavailable');
      gl.enableVertexAttribArray(position);
      gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);
      time = gl.getUniformLocation(program, 'u_time');
      color = gl.getUniformLocation(program, 'u_color');
      accent = resolvedAccent();
      gl.uniform3f(color, ...accent);
      const pixels = Math.round(40 * Math.min(window.devicePixelRatio || 1, 2));
      canvas.width = canvas.height = pixels;
      gl.viewport(0, 0, pixels, pixels);
      gl.uniform2f(gl.getUniformLocation(program, 'u_resolution'), pixels, pixels);
      canvas.style.visibility = 'visible';
    } catch { fail(); }
  }
  function canAnimate() {
    return active && !document.hidden && !reduce.matches && !forced.matches && !failed && !disposed;
  }
  function draw(now?: number) {
    frame = 0;
    if (!gl || failed || disposed || forced.matches || document.hidden) return;
    if (now !== undefined && previous !== null) elapsed += Math.min(now - previous, 100);
    previous = now ?? null;
    try {
      gl.uniform3f(color, ...accent);
      gl.uniform1f(time, reduce.matches ? 0 : elapsed / 1000);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
      if (gl.isContextLost() || gl.getError() !== gl.NO_ERROR) { fail(); return; }
    } catch { fail(); return; }
    if (canAnimate()) frame = requestAnimationFrame(draw);
  }
  function sync() {
    stop();
    if (!failed && !disposed) draw();
  }
  function lost(event: Event) {
    event.preventDefault();
    fail();
  }
  function restored() {
    if (disposed) return;
    failed = false;
    initialize();
    sync();
  }
  const themeObserver = new MutationObserver(() => {
    if (disposed) return;
    accent = resolvedAccent();
    // sync cancels the existing RAF and redraws even paused/reduced-motion
    // stills. Hidden/forced-color surfaces defer GPU work until resume.
    sync();
  });
  themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme', 'style'] });
  canvas.addEventListener('webglcontextlost', lost);
  canvas.addEventListener('webglcontextrestored', restored);
  document.addEventListener('visibilitychange', sync);
  reduce.addEventListener('change', sync);
  forced.addEventListener('change', sync);
  initialize();
  sync();
  return {
    setActive(value: boolean) { active = value; sync(); },
    dispose() {
      disposed = true;
      themeObserver.disconnect();
      release();
      canvas.removeEventListener('webglcontextlost', lost);
      canvas.removeEventListener('webglcontextrestored', restored);
      document.removeEventListener('visibilitychange', sync);
      reduce.removeEventListener('change', sync);
      forced.removeEventListener('change', sync);
    }
  };
}
