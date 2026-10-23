/**
 * The accessibility check every screen test runs: axe-core over the rendered page, the one copy
 * all the test files share.
 *
 * Only violations are asked for (`resultTypes`): axe then stops collecting every node that passed,
 * which is most of its work on a whole screen. Before this, nine files each ran the full collection,
 * and under a busy machine a test with several checks could run past its time limit.
 *
 * Colour contrast needs real layout, so it is off here and checked in a real browser instead.
 * `rules` adds to or overrides the defaults (the move page also turns off `region`).
 */
import axe from "axe-core";

export async function violations(rules: axe.RuleObject = {}): Promise<string[]> {
  const result = await axe.run(document.body, {
    resultTypes: ["violations"],
    rules: { "color-contrast": { enabled: false }, ...rules },
  });
  return result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
}
