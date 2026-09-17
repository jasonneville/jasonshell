import type { Extension } from '@codemirror/state';

type LanguageLoader = () => Promise<Extension>;

interface StackTextEditorLanguageInstallOptions {
  load?: (extension: string) => Promise<Extension | null>;
  apply: (language: Extension) => void;
  isDestroyed: () => boolean;
}

const languageLoaders: Record<string, LanguageLoader> = {
  'json': async () => (await import('@codemirror/lang-json')).json(),
  'js': async () => (await import('@codemirror/lang-javascript')).javascript(),
  'ts': async () => (await import('@codemirror/lang-javascript')).javascript({ typescript: true }),
  'svelte': async () => (await import('@replit/codemirror-lang-svelte')).svelte(),
  'css': async () => (await import('@codemirror/lang-css')).css(),
  'html': async () => (await import('@codemirror/lang-html')).html(),
  'xml': async () => (await import('@codemirror/lang-xml')).xml(),
  'yaml': async () => (await import('@codemirror/lang-yaml')).yaml(),
  'yml': async () => (await import('@codemirror/lang-yaml')).yaml(),
  'md': async () => (await import('@codemirror/lang-markdown')).markdown()
};

export function stackTextEditorExtension(path: string): string {
  const filename = path.split(/[\\/]/).at(-1) ?? path;
  const dotIndex = filename.lastIndexOf('.');
  return dotIndex >= 0 ? filename.slice(dotIndex + 1).toLowerCase() : '';
}

export function loadStackTextEditorLanguage(path: string): Promise<Extension | null> {
  const loader = languageLoaders[stackTextEditorExtension(path)];
  return loader ? loader() : Promise.resolve(null);
}

export async function installStackTextEditorLanguage(
  path: string,
  { load, apply, isDestroyed }: StackTextEditorLanguageInstallOptions
): Promise<void> {
  const extension = stackTextEditorExtension(path);
  if (!languageLoaders[extension]) return;

  try {
    const language = await (load ? load(extension) : loadStackTextEditorLanguage(path));
    if (isDestroyed() || !language) return;
    apply(language);
  } catch {
    // Syntax is optional; loader/parser failures must never affect editing.
  }
}
