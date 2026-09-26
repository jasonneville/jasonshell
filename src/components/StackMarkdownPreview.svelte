<script lang="ts">
  import { parseStackMarkdown, type MarkdownInline } from '../features/stack-browser/stackMarkdownPreview';
  export let source: string;
  $: blocks = parseStackMarkdown(source);
</script>

{#snippet inline(parts: MarkdownInline[])}{#each parts as part}{#if part.type === 'strong'}<strong>{part.text}</strong>{:else if part.type === 'code'}<code>{part.text}</code>{:else}{part.text}{/if}{/each}{/snippet}

<article class="markdown-preview" aria-label="Markdown preview">
  {#if blocks.length === 0}<p class="empty">Nothing to preview</p>{/if}
  {#each blocks as block}
    {#if block.type === 'heading'}
      {#if block.level === 1}<h1>{@render inline(block.content)}</h1>{:else if block.level === 2}<h2>{@render inline(block.content)}</h2>{:else}<h3>{@render inline(block.content)}</h3>{/if}
    {:else if block.type === 'paragraph'}<p>{@render inline(block.content)}</p>
    {:else if block.type === 'list'}
      {#if block.ordered}<ol>{#each block.items as item}<li>{@render inline(item)}</li>{/each}</ol>{:else}<ul>{#each block.items as item}<li>{@render inline(item)}</li>{/each}</ul>{/if}
    {:else if block.type === 'code'}<div class="code-frame">{#if block.language}<span>{block.language}</span>{/if}<pre><code>{block.code}</code></pre></div>
    {:else}<div class="table-frame"><table><thead><tr>{#each block.headers as cell, n}<th style:text-align={block.alignments[n] ?? undefined}>{@render inline(cell)}</th>{/each}</tr></thead><tbody>{#each block.rows as row}<tr>{#each row as cell, n}<td style:text-align={block.alignments[n] ?? undefined}>{@render inline(cell)}</td>{/each}</tr>{/each}</tbody></table></div>{/if}
  {/each}
</article>

<style>
  .markdown-preview{box-sizing:border-box;color:var(--js-color-text);font-family:Georgia,'Times New Roman',serif;line-height:1.72;max-width:80rem;min-height:100%;padding:clamp(1.5rem,5vw,4rem) clamp(1rem,4vw,2.5rem);width:100%}
  h1,h2,h3{color:var(--js-color-text-strong);font-family:'Trebuchet MS','Segoe UI',sans-serif;letter-spacing:-.025em;line-height:1.15} h1{border-bottom:1px solid var(--js-color-accent-border);font-size:clamp(2rem,6vw,3.5rem);margin:0 0 2rem;padding-bottom:1rem} h2{font-size:1.65rem;margin:2.5rem 0 .75rem} h3{font-size:1.15rem;margin:2rem 0 .6rem}
  p,ul,ol{margin:0 0 1.2rem} li{margin-block:.28rem;padding-left:.25rem} li::marker{color:var(--js-color-accent);font-weight:700}
  code{background:var(--js-color-accent-soft);border:1px solid var(--js-color-accent-border);border-radius:var(--js-radius-xs);font-family:'Cascadia Code',Consolas,monospace;font-size:.88em;padding:.12em .34em}.code-frame{background:var(--js-bg-surface);border:1px solid var(--js-color-border);border-radius:var(--js-radius-md);box-shadow:0 .8rem 2rem color-mix(in srgb,var(--js-color-text) 10%,transparent);margin:1.5rem 0;overflow:hidden}.code-frame>span{border-bottom:1px solid var(--js-color-border);color:var(--js-color-text-muted);display:block;font:600 .65rem/1 sans-serif;letter-spacing:.1em;padding:.65rem 1rem;text-transform:uppercase}pre{margin:0;max-width:100%;overflow:auto;padding:1rem}pre code{background:transparent;border:0;padding:0}
  .table-frame{border:1px solid var(--js-color-border);border-radius:var(--js-radius-md);margin:1.5rem 0;overflow:auto}table{border-collapse:collapse;min-width:100%}th,td{border-bottom:1px solid var(--js-color-border);padding:.65rem .85rem;text-align:left;white-space:nowrap}th{background:var(--js-color-accent-soft);color:var(--js-color-text-strong);font-family:'Trebuchet MS','Segoe UI',sans-serif}tbody tr:last-child td{border-bottom:0}.empty{color:var(--js-color-text-muted);font-style:italic;text-align:center}
</style>
