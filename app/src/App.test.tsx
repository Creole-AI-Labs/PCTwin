import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import App from "./App";

afterEach(cleanup);

it("shows the product name as the page heading", () => {
  render(<App />);
  expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("PCTwin");
});
