import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    // Accessibility checks over a whole screen take a few seconds on a busy machine; a slow run is
    // not a failure. Each waitFor keeps its own one-second limit, so a hang still fails fast.
    testTimeout: 20000,
    // The full move's screens are tested as they work; one test switches it off to see its
    // coming-soon page.
    env: { VITE_PCTWIN_FULL_MOVE: "1" },
  },
});
