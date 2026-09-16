import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { bracketMatching } from '@codemirror/language';
import { searchKeymap } from '@codemirror/search';
import { EditorState } from '@codemirror/state';
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
  onChange: (draft: string, dirty: boolean) => void;
  onDismiss: (dirty: boolean) => void;
}

export interface StackTextEditorAdapter {
  focusAtStart(): void;
  destroy(): void;
}

const stackEditorTheme = EditorView.theme({
  '&': {
    backgroundColor: 'transparent',
    color: 'var(--js-color-text-strong)',
    fontFamily: '"Cascadia Code", "Cascadia Mono", Consolas, monospace',
    fontSize: '0.78rem',
    height: '100%'
  },
  '&.cm-focused': { outline: 'var(--js-focus-ring)' },
  '.cm-scroller': { lineHeight: '1.55', overflow: 'auto' },
  '.cm-content': { caretColor: 'var(--js-color-text-strong)', padding: 'var(--js-space-3) 0' },
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

export function createStackTextEditorAdapter({
  parent,
  content,
  onChange,
  onDismiss
}: StackTextEditorAdapterOptions): StackTextEditorAdapter {
  let draft = content;
  let dirty = false;

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

  return {
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
      view.destroy();
    }
  };
}
