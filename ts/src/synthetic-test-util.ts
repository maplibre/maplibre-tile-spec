import { expect } from "vitest";

const RELATIVE_FLOAT_TOLERANCE = 0.0001 / 100;
const ABSOLUTE_FLOAT_TOLERANCE = Number.EPSILON;

export function expectJsonEqualWithTolerance(expectedJson: Record<any, any>, actualJson: Record<any, any>) {
    expect.addEqualityTesters([
        (received, expected) => {
            if (typeof received !== "number" || typeof expected !== "number") {
                return undefined;
            }

            // Handle Infinity/NaN
            if (!Number.isFinite(expected)) return Object.is(received, expected);

            // Handle Close to Zero
            if (Math.abs(expected) < ABSOLUTE_FLOAT_TOLERANCE) {
                return Math.abs(received) <= ABSOLUTE_FLOAT_TOLERANCE;
            }

            // Handle Relative Tolerance
            const relativeError = Math.abs(received - expected) / Math.abs(expected);
            return relativeError <= RELATIVE_FLOAT_TOLERANCE;
        },
    ]);
    expect(actualJson).toEqual(expectedJson);
}
