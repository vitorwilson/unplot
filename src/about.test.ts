import { describe, expect, it } from "vitest";
import { ISSUES_URL, REPO_URL, versionLabel } from "./about";

describe("versionLabel", () => {
  it("prefixes the version with a v", () => {
    expect(versionLabel("0.1.0")).toBe("v0.1.0");
    expect(versionLabel("1.2.3")).toBe("v1.2.3");
  });
});

describe("About links", () => {
  it("points at the project's GitHub repository", () => {
    expect(REPO_URL).toBe("https://github.com/vitorwilson/unplot");
  });

  it("derives the issue tracker from the repository", () => {
    expect(ISSUES_URL).toBe(`${REPO_URL}/issues`);
  });
});
