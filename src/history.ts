/**
 * Linear undo/redo over immutable snapshots. Holds a list of states and a cursor;
 * `push` after the cursor discards any redo branch (the standard editor model).
 *
 * @example
 * const h = new History<number>(0);
 * h.push(1);
 * h.undo(); // 0
 * h.redo(); // 1
 */
export class History<T> {
  private states: T[];
  private index = 0;

  constructor(initial: T) {
    this.states = [initial];
  }

  /** Record a new state as the present, discarding any redo future. */
  push(state: T): void {
    this.states = this.states.slice(0, this.index + 1);
    this.states.push(state);
    this.index = this.states.length - 1;
  }

  /** Step back one state (or stay at the oldest) and return it. */
  undo(): T {
    if (this.index > 0) {
      this.index -= 1;
    }
    return this.current;
  }

  /** Step forward one state (or stay at the newest) and return it. */
  redo(): T {
    if (this.index < this.states.length - 1) {
      this.index += 1;
    }
    return this.current;
  }

  get current(): T {
    return this.states[this.index];
  }

  get canUndo(): boolean {
    return this.index > 0;
  }

  get canRedo(): boolean {
    return this.index < this.states.length - 1;
  }
}
