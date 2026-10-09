export const SWATCH_TOKENS: readonly string[];
export function declaredIn(css: string): Record<string, string>;
export function swatchOf(css: string, base?: Record<string, string>): string[];
