export function isStackBasicTextEditorDirty(draft: string, persistedContent: string): boolean {
  return draft !== persistedContent;
}
