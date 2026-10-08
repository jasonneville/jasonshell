// Dependency-free decoder for Chromium's non-interlaced 8-bit RGB/RGBA PNGs.
import { readFileSync } from 'node:fs';
import { inflateSync } from 'node:zlib';
export function pngPixels(path) {
  const png = readFileSync(path);
  if (!png.subarray(0, 8).equals(Buffer.from([137,80,78,71,13,10,26,10]))) throw new Error('Not PNG');
  let width, height, channels;
  const chunks = [];
  for (let offset = 8; offset < png.length;) {
    const size = png.readUInt32BE(offset), type = png.toString('ascii', offset + 4, offset + 8);
    const data = png.subarray(offset + 8, offset + 8 + size);
    if (type === 'IHDR') {
      width = data.readUInt32BE(0); height = data.readUInt32BE(4);
      channels = data[9] === 2 ? 3 : data[9] === 6 ? 4 : 0;
      if (data[8] !== 8 || !channels || data[12] !== 0) throw new Error('Unsupported screenshot PNG encoding');
    }
    if (type === 'IDAT') chunks.push(data);
    offset += size + 12;
  }
  const raw = inflateSync(Buffer.concat(chunks)), stride = width * channels;
  if (raw.length !== (stride + 1) * height) throw new Error('Invalid screenshot scanlines');
  const pixels = Buffer.alloc(stride * height);
  const paeth = (a,b,c) => {
    const p=a+b-c, pa=Math.abs(p-a), pb=Math.abs(p-b), pc=Math.abs(p-c);
    return pa<=pb && pa<=pc ? a : pb<=pc ? b : c;
  };
  for (let y=0; y<height; y++) {
    const filter=raw[y*(stride+1)];
    if (filter>4) throw new Error('Invalid PNG filter');
    for (let x=0; x<stride; x++) {
      const a=x>=channels ? pixels[y*stride+x-channels] : 0;
      const b=y ? pixels[(y-1)*stride+x] : 0;
      const c=y && x>=channels ? pixels[(y-1)*stride+x-channels] : 0;
      const prediction=[0,a,b,Math.floor((a+b)/2),paeth(a,b,c)][filter];
      pixels[y*stride+x]=(raw[y*(stride+1)+x+1]+prediction)&255;
    }
  }
  return { width, height, pixel(x,y) {
    if (x<0 || y<0 || x>=width || y>=height) throw new Error('Pixel outside screenshot');
    return [...pixels.subarray((y*width+x)*channels,(y*width+x)*channels+3)];
  } };
}
