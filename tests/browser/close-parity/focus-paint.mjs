// Inputs are Chromium computed values, not authored CSS. Split shadow layers
// outside color functions so legacy rgba commas stay inside their layer.
function hasAlpha(color) {
  if (!color || color === 'transparent') return false;
  const alpha = color.match(/\/\s*([\d.]+)%?\s*\)$/)?.[1]
    ?? color.match(/^rgba\([^)]*,\s*([\d.]+)\s*\)$/)?.[1];
  return alpha === undefined || Number(alpha) > 0;
}

export function hasFocusPaint([outlineStyle, outlineWidth, outlineColor, boxShadow]) {
  if (!['none', 'hidden'].includes(outlineStyle) && parseFloat(outlineWidth) > 0 && hasAlpha(outlineColor)) return true;
  return boxShadow.split(/,(?![^()]*\))/).some(layer => {
    const color = layer.match(/[\w-]+\([^)]*\)|\btransparent\b/)?.[0];
    const [x = 0, y = 0, blur = 0, spread = 0] = (layer.match(/-?[\d.]+px/g) ?? []).map(parseFloat);
    return hasAlpha(color) && Math.max(Math.abs(x), Math.abs(y)) + blur + spread > 0;
  });
}
