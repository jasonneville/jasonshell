const CONFLICT_MESSAGES = new Set([
  'Text file changed on disk; reload before saving',
  'Text file changed during save; reload before saving',
  'Text file target changed while opening'
]);

/** Backend conflicts require a fresh baseline; other save errors may be retried. */
export function isStackBasicTextSaveConflict(message: string): boolean {
  return CONFLICT_MESSAGES.has(message);
}
