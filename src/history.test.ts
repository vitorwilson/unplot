import { describe, expect, it } from "vitest";
import { History } from "./history";

describe("History", () => {
  it("starts at the initial state with nothing to undo or redo", () => {
    const h = new History<string>("a");
    expect(h.current).toBe("a");
    expect(h.canUndo).toBe(false);
    expect(h.canRedo).toBe(false);
  });

  it("undoes and redoes across pushes", () => {
    const h = new History<string>("a");
    h.push("b");
    h.push("c");
    expect(h.current).toBe("c");
    expect(h.undo()).toBe("b");
    expect(h.undo()).toBe("a");
    expect(h.canUndo).toBe(false);
    expect(h.redo()).toBe("b");
    expect(h.redo()).toBe("c");
    expect(h.canRedo).toBe(false);
  });

  it("clamps at the ends", () => {
    const h = new History<number>(0);
    expect(h.undo()).toBe(0); // already oldest
    h.push(1);
    expect(h.redo()).toBe(1); // already newest
  });

  it("discards the redo branch after a new push", () => {
    const h = new History<string>("a");
    h.push("b");
    h.undo(); // back to "a"
    h.push("z"); // new branch from "a"
    expect(h.current).toBe("z");
    expect(h.canRedo).toBe(false);
    expect(h.undo()).toBe("a");
  });

  it("carries null as a valid state (empty canvas)", () => {
    const h = new History<number | null>(null);
    h.push(42);
    expect(h.undo()).toBeNull();
    expect(h.redo()).toBe(42);
  });
});
