#!/usr/bin/env node
/**
 * Generate binary fixtures for FastPFOR cross-language tests.
 * 
 * This script parses the C++ test file and outputs binary fixtures
 * that can be loaded by TypeScript tests without runtime C++ parsing.
 * 
 * Usage: node scripts/generate-fastpfor-fixtures.mjs
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const CPP_TEST_FILE = path.resolve(__dirname, '../cpp/test/test_fastpfor.cpp');
const FIXTURES_DIR = path.resolve(__dirname, '../test/fixtures/fastpfor');

/**
 * Parse a C++ uint32_t array from source code.
 * Returns Uint32Array to preserve bit-exact representation of unsigned values.
 */
function parseCppUint32Array(source, name) {
    const re = new RegExp(
        String.raw`\b(?:static\s+)?(?:constexpr\s+)?const\s+(?:std::)?uint32_t\s+${name}\s*\[\]\s*=\s*\{([\s\S]*?)\};`,
        'm',
    );
    const match = re.exec(source);
    if (!match) {
        throw new Error(`Failed to locate C++ array ${name}`);
    }

    // Strip C++ comments for robust parsing
    const body = match[1]
        .replace(/\/\/.*$/gm, '')
        .replace(/\/\*[\s\S]*?\*\//g, '');

    const tokens = body
        .split(',')
        .map((t) => t.trim())
        .filter((t) => t.length > 0);

    const values = new Uint32Array(tokens.length);
    for (let i = 0; i < tokens.length; i++) {
        let token = tokens[i];
        token = token.replace(/u$/i, '');
        token = token.replace(/^(?:UINT32_C|INT32_C)\((.*)\)$/, '$1');

        // Handle negative unsigned values like -100u
        if (token.startsWith('-')) {
            const num = Number(token);
            values[i] = num >>> 0; // Convert to unsigned
        } else if (token.startsWith('0x') || token.startsWith('0X')) {
            values[i] = Number.parseInt(token, 16) >>> 0;
        } else {
            values[i] = Number(token) >>> 0;
        }

        if (!Number.isFinite(values[i])) {
            throw new Error(`Failed to parse token '${tokens[i]}' in ${name}`);
        }
    }

    return values;
}

/**
 * Write Uint32Array as big-endian binary file.
 */
function writeBigEndianBinary(filepath, data) {
    const buffer = Buffer.alloc(data.length * 4);
    for (let i = 0; i < data.length; i++) {
        buffer.writeUInt32BE(data[i], i * 4);
    }
    fs.writeFileSync(filepath, buffer);
    console.log(`  Written: ${path.basename(filepath)} (${data.length} values, ${buffer.length} bytes)`);
}

function main() {
    console.log('Generating FastPFOR binary fixtures...\n');

    // Read C++ source
    const cppSource = fs.readFileSync(CPP_TEST_FILE, 'utf8');
    console.log(`Read: ${CPP_TEST_FILE}\n`);

    // Create output directory
    fs.mkdirSync(FIXTURES_DIR, { recursive: true });

    // Generate fixtures for each vector (1-4)
    for (let i = 1; i <= 4; i++) {
        console.log(`Vector ${i}:`);

        const uncompressed = parseCppUint32Array(cppSource, `uncompressed${i}`);
        const compressed = parseCppUint32Array(cppSource, `compressed${i}`);

        writeBigEndianBinary(
            path.join(FIXTURES_DIR, `vector${i}_uncompressed.bin`),
            uncompressed
        );
        writeBigEndianBinary(
            path.join(FIXTURES_DIR, `vector${i}_compressed.bin`),
            compressed
        );
    }

    // Write README
    const readme = `# FastPFOR Test Fixtures

Binary fixtures for cross-language FastPFOR validation tests.

## Format

Each \`.bin\` file contains a sequence of 32-bit unsigned integers in **big-endian** byte order.

## Files

| File | Description |
|------|-------------|
| \`vector{N}_uncompressed.bin\` | Original uncompressed data (Int32Array view) |
| \`vector{N}_compressed.bin\` | FastPFOR-encoded data from C++ reference implementation |

## Regeneration

To regenerate these fixtures from the C++ source:

\`\`\`bash
node scripts/generate-fastpfor-fixtures.mjs
\`\`\`

## Source

Generated from \`cpp/test/test_fastpfor.cpp\`.
`;

    fs.writeFileSync(path.join(FIXTURES_DIR, 'README.md'), readme);
    console.log(`\nWritten: README.md`);

    console.log('\nDone!');
}

main();
