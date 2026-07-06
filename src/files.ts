import { open, save } from "@tauri-apps/plugin-dialog";
import { openCurve, saveCurve, type FittedCurve, type Knot } from "./fit";

// Save/open a drawn curve as a `.unplot` file (Phase 6). The native dialog picks
// the path; the Rust command does the I/O and (de)serialization through the core
// document format, so the file stores only the source-of-truth knots.

const UNPLOT_FILTER = [{ name: "unplot curve", extensions: ["unplot"] }];
const DEFAULT_NAME = "curve.unplot";

/** Prompt for a destination and save the curve there. Resolves `false` if the
 * user cancels the dialog. */
export async function saveCurveDialog(knots: Knot[]): Promise<boolean> {
  const path = await save({
    filters: UNPLOT_FILTER,
    defaultPath: DEFAULT_NAME,
  });
  if (!path) {
    return false;
  }
  await saveCurve(path, knots);
  return true;
}

/** Prompt for a `.unplot` file and load it. Resolves `null` if the user cancels. */
export async function openCurveDialog(): Promise<FittedCurve | null> {
  const selected = await open({
    filters: UNPLOT_FILTER,
    multiple: false,
    directory: false,
  });
  if (typeof selected !== "string") {
    return null;
  }
  return openCurve(selected);
}
