const UNITS = ["B", "KiB", "MiB", "GiB"];

/** A byte count in the largest binary unit that keeps it at or above one, to one decimal. */
export function formatBytes(len: number): string {
  let value = len;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  if (unit === 0) return `${len} B`;
  return `${value.toFixed(1)} ${UNITS[unit]}`;
}
