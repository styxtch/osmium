export interface TextToken {
  id: string;
  text: string;
  isWord: boolean;
  start: number;
  end: number;
}

export interface TextStats {
  chars: number;
  sentences: number;
  paragraphs: number;
}

const WORD_REGEX = /[\p{L}\p{N}]+(?:[-'][\p{L}\p{N}]+)*/gu;

export function tokenizeIndonesianText(text: string): TextToken[] {
  if (!text) return [];
  const tokens: TextToken[] = [];
  let lastIdx = 0;

  for (const match of text.matchAll(WORD_REGEX)) {
    const start = match.index ?? 0;
    const word = match[0];
    if (start > lastIdx) {
      tokens.push({
        id: `sep-${lastIdx}`,
        text: text.slice(lastIdx, start),
        isWord: false,
        start: lastIdx,
        end: start,
      });
    }
    const end = start + word.length;
    tokens.push({
      id: `word-${start}`,
      text: word,
      isWord: true,
      start,
      end,
    });
    lastIdx = end;
  }

  if (lastIdx < text.length) {
    tokens.push({
      id: `sep-${lastIdx}`,
      text: text.slice(lastIdx),
      isWord: false,
      start: lastIdx,
      end: text.length,
    });
  }

  return tokens;
}

export function extractSentenceContext(
  text: string,
  start: number,
  end: number
): string {
  const safeStart = Math.max(0, Math.min(start, text.length));
  const safeEnd = Math.max(safeStart, Math.min(end, text.length));

  let left = safeStart;
  while (left > 0) {
    const ch = text[left - 1];
    if (ch === "\n" || ch === "." || ch === "!" || ch === "?") {
      break;
    }
    left -= 1;
  }

  let right = safeEnd;
  while (right < text.length) {
    const ch = text[right];
    right += 1;
    if (ch === "\n" || ch === "." || ch === "!" || ch === "?") {
      break;
    }
  }

  return text.slice(left, right).trim() || text.trim();
}

export function matchCapitalization(
  original: string,
  replacement: string
): string {
  if (!original || !replacement) return replacement;

  const isAllUpper =
    original.length > 1 &&
    original === original.toUpperCase() &&
    original !== original.toLowerCase();
  if (isAllUpper) {
    return replacement.toUpperCase();
  }

  const origFirst = original[0];
  const origStartsUpper =
    origFirst === origFirst.toUpperCase() &&
    origFirst !== origFirst.toLowerCase();

  if (origStartsUpper) {
    return replacement[0].toUpperCase() + replacement.slice(1);
  }

  const repFirst = replacement[0];
  const repStartsUpper =
    repFirst === repFirst.toUpperCase() && repFirst !== repFirst.toLowerCase();
  if (repStartsUpper && replacement === replacement[0] + replacement.slice(1).toLowerCase()) {
    return replacement[0].toLowerCase() + replacement.slice(1);
  }

  return replacement;
}

export function replaceTokenAtOffset(
  text: string,
  start: number,
  end: number,
  replacement: string
): string {
  const original = text.slice(start, end);
  const adapted = matchCapitalization(original, replacement);
  return text.slice(0, start) + adapted + text.slice(end);
}

export function computeTextStats(text: string): TextStats {
  const chars = Array.from(text).length;
  const trimmed = text.trim();
  if (!trimmed) {
    return { chars, sentences: 0, paragraphs: 0 };
  }

  const paragraphs = trimmed
    .split(/\r?\n\s*\r?\n/)
    .filter((p) => p.trim().length > 0).length;

  const sentences = trimmed
    .split(/[.!?]+(?:\s+|$)/)
    .filter((s) => s.trim().length > 0).length;

  return {
    chars,
    sentences: Math.max(1, sentences),
    paragraphs: Math.max(1, paragraphs),
  };
}
