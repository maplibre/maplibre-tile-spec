/** How a decoded blob reads, shared by the detail pane and the hover tip. */

import type { DecodedBlob } from "./annotate.ts";

/** The values of a blob, one chip each, with text cut to `maxChars`. */
export function blobChips(blob: DecodedBlob, maxChars: number): string[] {
  switch (blob.kind) {
    case "numbers":
    case "bigints":
      return blob.values.map(String);
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
  return blob.truncatedFrom === null
    ? `${blob.values.length} values`
    : `${blob.values.length} of ${blob.truncatedFrom} values`;
}
