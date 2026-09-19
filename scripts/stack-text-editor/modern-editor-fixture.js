import { json } from '@codemirror/lang-json';
import { codeFolding, foldedRanges, foldGutter, foldKeymap } from '@codemirror/language';
import { undo } from '@codemirror/commands';
import { EditorState } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import { createStackFoldMarker } from '../../src/features/stack-browser/stackFoldControl.ts';
import { createStackTextEditorAdapter } from '../../src/features/stack-browser/stackTextEditorAdapter.ts';

if (typeof document !== 'undefined') {
  const results = document.querySelector('#results');
  const beforeDocument = '{\n  "nested": {"value": 1}\n}\n';
  let destroyed = false;
  let view;
  const position = control => {
    const element = control.closest('.cm-gutterElement');
    const top = element.getBoundingClientRect().top - view.scrollDOM.getBoundingClientRect().top + view.scrollDOM.scrollTop;
    return view.lineBlockAtHeight(top + 1).from;
  };
  const restore = control => {
    const from = position(control);
    control.dataset.foldFrom = String(from);
    requestAnimationFrame(() => requestAnimationFrame(() => {
      if (destroyed) return;
      for (const replacement of view.dom.querySelectorAll('.stack-editor-fold-control')) {
        const candidate = position(replacement);
        replacement.dataset.foldFrom = String(candidate);
        if (candidate === from) replacement.focus();
      }
    }));
  };
  const extensions = [codeFolding(), foldGutter({ markerDOM: open => createStackFoldMarker(open, () => destroyed, restore) }), keymap.of(foldKeymap)];
  view = new EditorView({
    parent: document.querySelector('#editor'),
    state: EditorState.create({ doc: beforeDocument, extensions: [json(), extensions] })
  });
  const xmlBefore = '<x:item id="1"';
  let dirtyCalls = 0;
  const xmlParent = document.querySelector('#xml-editor');
  const xmlAdapter = createStackTextEditorAdapter({
    parent: xmlParent,
    content: xmlBefore,
    path: 'fixture.xml',
    fontStack: 'monospace',
    onChange: () => { dirtyCalls++; },
    onDismiss: () => {}
  });
  requestAnimationFrame(() => {
    const control = document.querySelector('button[aria-expanded]');
    const foldFrom = position(control);
    control?.focus();
    control?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    requestAnimationFrame(() => requestAnimationFrame(() => {
      const replacement = document.activeElement;
      const afterDocument = view.state.doc.toString();
      const folded = foldedRanges(view.state).size > 0;
      const foldPass = folded && replacement?.dataset.foldFrom === String(foldFrom) && replacement?.getAttribute('aria-label') === 'Expand code block' && replacement?.getAttribute('aria-expanded') === 'false' && beforeDocument === afterDocument;
      const xmlView = EditorView.findFromDOM(xmlParent);
      xmlView.contentDOM.focus();
      document.execCommand('insertText', false, '>');
      const completedXml = xmlView.state.doc.toString();
      const undone = undo(xmlView);
      const xmlPass = completedXml === '<x:item id="1"></x:item>' && dirtyCalls === 1 && undone && xmlView.state.doc.toString() === xmlBefore;
      const pass = foldPass && xmlPass;
      results.textContent = pass ? 'PASS: real CM keyboard fold + XML input/listener/undo.' : `FAIL: fold=${foldPass} xml=${xmlPass}`;
      results.dataset.folded = String(folded);
      results.dataset.foldFrom = replacement?.dataset.foldFrom ?? '';
      results.dataset.documentUnchanged = String(beforeDocument === afterDocument);
      results.dataset.xmlCompleted = completedXml;
      results.dataset.xmlDirtyCalls = String(dirtyCalls);
      results.dataset.xmlUndo = xmlView.state.doc.toString();
    }));
  });
}
