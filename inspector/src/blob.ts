/** How a decoded blob reads, shared by the detail pane and the hover tip. */

import type { DecodedBlob } from "./annotate.ts";

/** The values of a blob, one chip each, with text cut to `maxChars`. */
export function blobChips(blob: DecodedBlob, maxChars: number): string[] {
  switch (blob.kind) {
    case "numbers":
    case "bigints":
      return blob.values.map(String);
    case "strings":
      return blob.values;
    case "enum":
      return blob.names;
    case "bools":
      return blob.values.map((value) => (value ? "1" : "0"));
    case "text":
      return [[...blob.value].slice(0, maxChars).join("")];
    case "binary":
      return [`${blob.len} binary bytes`];
    case "error":
      return [blob.message];
  }
}

/** A chip, and how many identical ones in a row it stands for. */
export interface Run {
  chip: string;
  count: number;
  /** Index of the first chip it covers, which a line of the same values beneath it shares. */
  from: number;
}

/** Fewer repeats than this read better written out than as a count. */
export const MIN_RUN = 4;

/** Folds each run of at least `MIN_RUN` identical chips into one, leaving shorter ones as they are. */
export function runs(chips: string[]): Run[] {
  const out: Run[] = [];
  let at = 0;
  while (at < chips.length) {
    let end = at + 1;
    while (end < chips.length && chips[end] === chips[at]) end++;
    const count = end - at;
    if (count >= MIN_RUN) out.push({ chip: chips[at], count, from: at });
    else
      for (let i = at; i < end; i++)
        out.push({ chip: chips[i], count: 1, from: i });
    at = end;
  }
  return out;
}

/** The numbers a named value was stored as, which the names above them stand in for. */
export function blobRaw(blob: DecodedBlob): string[] | null {
  return blob.kind === "enum" ? blob.values.map(String) : null;
}

/** What is cut from the end: characters for text, values for the rest, `null` when nothing is. */
export function blobHidden(
  blob: DecodedBlob,
  maxChars: number,
): { count: number; unit: "chars" | "values" | "strings" } | null {
  if (blob.kind === "text") {
    const count = [...blob.value].length - maxChars;
    return count > 0 ? { count, unit: "chars" } : null;
  }
  if (blob.kind === "error" || blob.kind === "binary") return null;
  return blob.truncatedFrom === null
    ? null
    : {
        count: blob.truncatedFrom - blob.values.length,
        unit: blob.kind === "strings" ? "strings" : "values",
      };
}

/** What the chips leave out, which is a character count for text and a value count for the rest. */
export function blobNote(blob: DecodedBlob, maxChars: number): string {
  if (blob.kind === "error") return "undecodable";
  if (blob.kind === "binary") return blob.kind;
  if (blob.kind === "text") {
    const chars = [...blob.value].length;
    return chars > maxChars
      ? `text, ${maxChars} of ${chars} chars`
      : `text, ${chars} chars`;
  }
  const noun = blob.kind === "strings" ? "string" : "value";
  const total = blob.truncatedFrom ?? blob.values.length;
  const unit = total === 1 ? noun : `${noun}s`;
  return blob.truncatedFrom === null
    ? `${blob.values.length} ${unit}`
    : `${blob.values.length} of ${blob.truncatedFrom} ${unit}`;
}
