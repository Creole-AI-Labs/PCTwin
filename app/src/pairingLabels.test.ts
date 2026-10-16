import { describe, expect, it } from "vitest";
import { localization } from "./i18n";
import { labelText as rawLabelText } from "./pairing";

// Fluent wraps each value in invisible direction marks; compare the visible words.
const labelText = (...args: Parameters<typeof rawLabelText>) => rawLabelText(...args).replace(/[⁦-⁩]/g, "");

const label = (colour: string, animal: string) => ({ kind: "picked", colour, animal }) as const;

describe("device labels in each language", () => {
  it("English keeps colour then animal", () => {
    expect(labelText(localization("en"), label("blue", "fox"))).toBe("Blue Fox");
  });

  it("French puts the animal first and the colour agrees with it", () => {
    const fr = localization("fr");
    expect(labelText(fr, label("blue", "fox"))).toBe("Renard bleu");
    expect(labelText(fr, label("blue", "owl"))).toBe("Chouette bleue");
  });

  it("Hindi keeps colour first and the colour agrees with the animal", () => {
    const hi = localization("hi");
    expect(labelText(hi, label("blue", "fox"))).toBe("नीली लोमड़ी");
    expect(labelText(hi, label("blue", "owl"))).toBe("नीला उल्लू");
  });

  it("Chinese joins colour and animal without a space", () => {
    expect(labelText(localization("zh-CN"), label("blue", "fox"))).toBe("蓝色狐狸");
  });

  it("every colour and animal reads cleanly in every language", () => {
    const colours = ["red", "orange", "yellow", "green", "blue", "purple", "pink", "brown", "black", "white", "grey", "gold"];
    const animals = ["fox", "owl", "ant", "lion", "zebra", "bat"];
    for (const code of ["en", "fr", "es", "pt", "ar", "hi", "zh-CN"] as const) {
      const l10n = localization(code);
      for (const c of colours) {
        for (const a of animals) {
          const text = labelText(l10n, label(c, a));
          expect(text, `${code} ${c} ${a}`).not.toMatch(/label-|[{}$]/);
          expect(text.trim()).not.toBe("");
        }
      }
    }
  });
});
