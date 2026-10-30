/**
 * Names from the laptops, made safe to show (third review, finding 10). A file, folder, app or
 * person's name can hold the invisible characters that change the direction text is drawn in (a
 * right-to-left override, say), so "photo" + RLO + "gpj.exe" would show as "photoexe.jpg". Each such
 * character is shown as a visible replacement mark instead, so what is shown is what is there.
 * Letters of every language, right-to-left scripts included, are left as they are; each name is
 * still isolated for direction where it is drawn (`<bdi>`).
 */

/** The characters that change text direction: marks, embeddings and overrides. */
const DIRECTION_CONTROLS = /[\u061C\u200E\u200F\u202A-\u202E]/g;
/**
 * The isolates (U+2066 to U+2069): removed, never shown. They only fence text off and cannot
 * reorder what is outside them, and translation libraries put them around every name they insert,
 * so marking them drew "\uFFFD" around ordinary laptop names (the founder, 30 October 2026).
 */
const ISOLATES = /[\u2066-\u2069]/g;

/** The mark shown in place of a direction character. */
export const MARK = "\uFFFD";

/** A name as it is shown: every direction character replaced by a visible mark. */
export function shownName(name: string): string {
  return name.replace(ISOLATES, "").replace(DIRECTION_CONTROLS, MARK);
}

/** Every piece of text in `v` (objects and lists, to any depth) as it is shown. */
export function shownNames<T>(v: T): T {
  if (typeof v === "string") return shownName(v) as T;
  if (Array.isArray(v)) return v.map((x: unknown) => shownNames(x)) as T;
  if (v !== null && typeof v === "object") {
    return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, shownNames(x)])) as T;
  }
  return v;
}
