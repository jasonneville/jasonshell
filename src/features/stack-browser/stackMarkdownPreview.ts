export type MarkdownInline = { type: 'text' | 'strong' | 'code'; text: string };
export type MarkdownBlock =
  | { type: 'heading'; level: 1 | 2 | 3; content: MarkdownInline[] }
  | { type: 'paragraph'; content: MarkdownInline[] }
  | { type: 'list'; ordered: boolean; items: MarkdownInline[][] }
  | { type: 'code'; language: string; code: string }
  | { type: 'table'; headers: MarkdownInline[][]; rows: MarkdownInline[][][]; alignments: Array<'left' | 'center' | 'right' | null> };

export function parseMarkdownInline(value: string): MarkdownInline[] {
  const parts: MarkdownInline[] = [];
  const pattern = /(\*\*([^*]+)\*\*|`([^`]+)`)/g;
  let cursor = 0;
  for (const match of value.matchAll(pattern)) {
    const index = match.index ?? 0;
    if (index > cursor) parts.push({ type: 'text', text: value.slice(cursor, index) });
    parts.push(match[2] !== undefined ? { type: 'strong', text: match[2] } : { type: 'code', text: match[3] });
    cursor = index + match[0].length;
  }
  if (cursor < value.length) parts.push({ type: 'text', text: value.slice(cursor) });
  return parts;
}

const cells = (line: string) => line.trim().replace(/^\|/, '').replace(/\|$/, '').split('|').map((cell) => cell.trim());
const separator = (line: string) => cells(line).every((cell) => /^:?-{3,}:?$/.test(cell));
function alignment(cell: string): 'left' | 'center' | 'right' | null {
  if (/^:-+:$/.test(cell)) return 'center';
  if (/^-+:$/.test(cell)) return 'right';
  if (/^:-+$/.test(cell)) return 'left';
  return null;
}

export function parseStackMarkdown(source: string): MarkdownBlock[] {
  const lines = source.replace(/\r\n?/g, '\n').split('\n');
  const blocks: MarkdownBlock[] = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (!line.trim()) { i++; continue; }
    const fence = line.match(/^\s*```\s*([^\s`]*)\s*$/);
    if (fence) {
      const code: string[] = []; i++;
      while (i < lines.length && !/^\s*```\s*$/.test(lines[i])) code.push(lines[i++]);
      if (i < lines.length) i++;
      blocks.push({ type: 'code', language: fence[1] ?? '', code: code.join('\n') }); continue;
    }
    const heading = line.match(/^(#{1,3})\s+(.+)$/);
    if (heading) { blocks.push({ type: 'heading', level: heading[1].length as 1 | 2 | 3, content: parseMarkdownInline(heading[2].trim()) }); i++; continue; }
    if (line.includes('|') && i + 1 < lines.length && separator(lines[i + 1])) {
      const headers = cells(line).map(parseMarkdownInline);
      const alignments = cells(lines[i + 1]).map(alignment);
      const rows: MarkdownInline[][][] = []; i += 2;
      while (i < lines.length && lines[i].trim() && lines[i].includes('|')) rows.push(cells(lines[i++]).map(parseMarkdownInline));
      blocks.push({ type: 'table', headers, rows, alignments }); continue;
    }
    const item = line.match(/^\s*(?:([-+*])|(\d+)[.)])\s+(.+)$/);
    if (item) {
      const ordered = Boolean(item[2]); const items: MarkdownInline[][] = [];
      while (i < lines.length) {
        const next = lines[i].match(/^\s*(?:([-+*])|(\d+)[.)])\s+(.+)$/);
        if (!next || Boolean(next[2]) !== ordered) break;
        items.push(parseMarkdownInline(next[3])); i++;
      }
      blocks.push({ type: 'list', ordered, items }); continue;
    }
    const paragraph = [line.trim()]; i++;
    while (i < lines.length && lines[i].trim()) {
      if (/^(#{1,3})\s+|^\s*```|^\s*(?:[-+*]|\d+[.)])\s+/.test(lines[i])) break;
      if (lines[i].includes('|') && i + 1 < lines.length && separator(lines[i + 1])) break;
      paragraph.push(lines[i++].trim());
    }
    blocks.push({ type: 'paragraph', content: parseMarkdownInline(paragraph.join(' ')) });
  }
  return blocks;
}
