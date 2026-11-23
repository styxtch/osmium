import { describe, expect, it } from "vitest";
import {
  computeTextStats,
  extractSentenceContext,
  matchCapitalization,
  replaceTokenAtOffset,
  tokenizeIndonesianText,
} from "../tokenizer";

describe("Indonesian text tokenizer & offset replacement", () => {
  it("keeps hyphenated Indonesian reduplication as a single word token", () => {
    const tokens = tokenizeIndonesianText("Mereka jalan-jalan dan berhati-hati.");
    const words = tokens.filter((t) => t.isWord).map((t) => t.text);
    expect(words).toEqual(["Mereka", "jalan-jalan", "dan", "berhati-hati"]);
  });

  it("replaces strictly at the clicked offset without touching duplicate words", () => {
    const text = "Langkah berani itu langkah yang sangat berani.";
    const tokens = tokenizeIndonesianText(text).filter(
      (t) => t.isWord && t.text === "berani"
    );
    expect(tokens).toHaveLength(2);

    const updated = replaceTokenAtOffset(
      text,
      tokens[1].start,
      tokens[1].end,
      "tegas"
    );
    expect(updated).toBe("Langkah berani itu langkah yang sangat tegas.");
  });

  it("preserves original capitalization (TitleCase, UPPERCASE, lowercase)", () => {
    expect(matchCapitalization("Berani", "tegas")).toBe("Tegas");
    expect(matchCapitalization("BERANI", "tegas")).toBe("TEGAS");
    expect(matchCapitalization("berani", "Tegas")).toBe("tegas");
  });

  it("extracts the enclosing sentence context and computes stats accurately", () => {
    const text = "Kalimat pertama. Keputusan berani diambil hari ini!\n\nParagraf dua.";
    const idx = text.indexOf("berani");
    const ctx = extractSentenceContext(text, idx, idx + "berani".length);
    expect(ctx).toBe("Keputusan berani diambil hari ini!");

    const stats = computeTextStats(text);
    expect(stats.paragraphs).toBe(2);
    expect(stats.sentences).toBe(3);
    expect(stats.chars).toBe(Array.from(text).length);
  });
});
