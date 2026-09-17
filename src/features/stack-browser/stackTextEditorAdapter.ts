import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { bracketMatching, HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { searchKeymap } from '@codemirror/search';
import { Compartment, EditorState } from '@codemirror/state';
import { tags } from '@lezer/highlight';
import { installStackTextEditorLanguage } from './stackTextEditorLanguages';
import {
  drawSelection,
  EditorView,
  highlightActiveLine,
  keymap,
  lineNumbers
} from '@codemirror/view';

export interface StackTextEditorAdapterOptions {
  parent: HTMLElement;
  content: string;
  path: string;
  fontStack: string;
  onChange: (draft: string, dirty: boolean) => void;
  onDismiss: (dirty: boolean) => void;
}

export interface StackTextEditorAdapter {
  focusAtStart(): void;
  setFont(fontStack: string): void;
  destroy(): void;
}

const calmEditorForeground = 'color-mix(in srgb, var(--js-color-text) 82%, var(--js-bg-surface))';
const calmEditorCaret = 'color-mix(in srgb, var(--js-color-text-strong) 84%, var(--js-bg-surface))';
const calmEditorAccent = 'color-mix(in srgb, var(--js-color-accent) 72%, var(--js-color-text))';
const calmEditorMuted = 'color-mix(in srgb, var(--js-color-text) 72%, var(--js-bg-surface))';

export const stackEditorTheme = EditorView.theme({
  '&': {
    backgroundColor: 'transparent',
    color: calmEditorForeground,
    fontSize: '0.78rem',
    height: '100%'
  },
  '&.cm-focused': { outline: 'var(--js-focus-ring)' },
  '.cm-scroller': { lineHeight: '1.55', overflow: 'auto' },
  '.cm-content': { caretColor: calmEditorCaret, padding: 'var(--js-space-3) 0' },
  '.cm-cursor': {
    borderLeft: '0',
    backgroundColor: calmEditorCaret,
    width: '0.62em'
  },
  '.cm-line': { padding: '0 var(--js-space-3)' },
  '.cm-gutters': {
    backgroundColor: 'var(--js-bg-surface)',
    borderRight: '1px solid var(--js-color-border)',
    color: 'var(--js-color-text-muted)'
  },
  '.cm-activeLine, .cm-activeLineGutter': { backgroundColor: 'var(--js-color-accent-soft)' },
  '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': {
    backgroundColor: 'var(--js-color-accent-border)'
  }
});

export function stackEditorFontTheme(fontStack: string) {
  return EditorView.theme({ '.cm-scroller': { fontFamily: fontStack } });
}

const stackHighlightStyle = HighlightStyle.define([
  { tag: tags.keyword, color: calmEditorAccent },
  { tag: [tags.name, tags.propertyName, tags.attributeName], color: calmEditorForeground },
  { tag: [tags.string, tags.inserted], color: calmEditorAccent },
  { tag: [tags.number, tags.bool, tags.null], color: calmEditorForeground, fontWeight: '600' },
  { tag: [tags.comment, tags.meta], color: calmEditorMuted, fontStyle: 'italic' },
  { tag: [tags.typeName, tags.className, tags.tagName], color: calmEditorAccent, fontWeight: '600' },
  { tag: [tags.operator, tags.punctuation], color: calmEditorMuted },
  { tag: tags.invalid, color: calmEditorForeground, textDecoration: 'underline wavy' }
]);

export function createStackTextEditorAdapter({
  parent,
  content,
  path,
  fontStack,
  onChange,
  onDismiss
}: StackTextEditorAdapterOptions): StackTextEditorAdapter {
  let draft = content;
  let dirty = false;
  let destroyed = false;
  const languageCompartment = new Compartment();
  const fontCompartment = new Compartment();

  const state = EditorState.create({
    doc: content,
    selection: { anchor: 0 },
    extensions: [
      history(),
      keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
      lineNumbers(),
      highlightActiveLine(),
      drawSelection(),
      bracketMatching(),
      syntaxHighlighting(stackHighlightStyle),
      languageCompartment.of([]),
      fontCompartment.of(stackEditorFontTheme(fontStack)),
      EditorView.contentAttributes.of({ 'aria-label': 'File contents', spellcheck: 'false' }),
      EditorView.updateListener.of((update) => {
        if (!update.docChanged) return;
        draft = update.state.doc.toString();
        dirty = draft !== content;
        onChange(draft, dirty);
      }),
      EditorView.domEventHandlers({
        keydown(event) {
          if (event.key !== 'Escape') return false;
          event.preventDefault();
          event.stopPropagation();
          onDismiss(dirty);
          return true;
        }
      }),
      stackEditorTheme
    ]
  });

  const view = new EditorView({ state, parent });

  void installStackTextEditorLanguage(path, {
    apply: (language) => {
      view.dispatch({ effects: languageCompartment.reconfigure(language) });
    },
    isDestroyed: () => destroyed
  });

  return {
    setFont(fontStack: string) {
      if (destroyed) return;
      view.dispatch({ effects: fontCompartment.reconfigure(stackEditorFontTheme(fontStack)) });
    },
    focusAtStart() {
      view.dispatch({
        selection: { anchor: 0 },
        scrollIntoView: true
      });
      view.focus();
      view.scrollDOM.scrollTop = 0;
      view.scrollDOM.scrollLeft = 0;
    },
    destroy() {
      destroyed = true;
      view.destroy();
    }
  };
}
