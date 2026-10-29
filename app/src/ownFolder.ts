/**
 * Several old laptops into one (2.6): the name of the folder that keeps one old laptop's things
 * apart on the new laptop ("From Bea's laptop"), in the person's language.
 *
 * Fluent wraps each placeable in invisible direction marks (U+2068 ... U+2069) so a name reads the
 * right way round on screen. In a folder name those marks would be real characters, so they are
 * taken out. A laptop name with a character no folder name may hold (on Windows, a Mac or Linux),
 * or none at all, gives the plain form ("From the other laptop") instead of a changed name.
 */
type GetString = (id: string, args?: Record<string, string>) => string;

/** Fluent's isolation marks and the other invisible direction marks. */
const DIRECTION_MARKS = /[⁦-⁩‎‏‪-‮؜]/g;

/** Characters a folder name may not hold on one of the systems PCTwin moves to, and control characters. */
// eslint-disable-next-line no-control-regex
const NOT_IN_A_FOLDER = /[<>:"/\\|?*\u0000-\u001f\u007f]/;

/** Whether `name` can be part of a folder name as it is. */
export function folderSafe(name: string): boolean {
  const n = name.replace(DIRECTION_MARKS, "");
  return n.trim() !== "" && n === n.trim() && !NOT_IN_A_FOLDER.test(n) && !n.endsWith(".") && n.length <= 100;
}

/** The own folder's name for the old laptop called `laptop` (null when it gave no name). */
export function ownFolderName(getString: GetString, laptop: string | null): string {
  const named = laptop !== null && folderSafe(laptop);
  const text = named ? getString("plan-own-folder-name", { laptop: laptop.replace(DIRECTION_MARKS, "") }) : getString("plan-own-folder-name-unnamed");
  const name = text.replace(DIRECTION_MARKS, "").trim();
  // A translation that itself breaks a folder name falls back to the plain form, then to nothing.
  if (folderSafe(name)) return name;
  const plain = getString("plan-own-folder-name-unnamed").replace(DIRECTION_MARKS, "").trim();
  return folderSafe(plain) ? plain : "PCTwin";
}
