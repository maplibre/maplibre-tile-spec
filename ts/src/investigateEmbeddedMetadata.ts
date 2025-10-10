import decodeTile from "./mltDecoder";
import * as fs from "node:fs";
import * as path from "node:path";

const MLT_DATA_DIR = "../../test/expected/tag0x01/omt";

export function investigateEmbeddedMetadata(filePath: string) {
	console.log(`\n========== ${path.basename(filePath)} ==========`);
	try {
		const bytes = new Uint8Array(fs.readFileSync(filePath));
		const tables = decodeTile(bytes);
		for (const t of tables) {
			const featureCount = t.numFeatures;
			let geomInfo = "unknown";
			if (featureCount > 0 && t.geometryVector && typeof t.geometryVector.geometryType === "function") {
				try {
					const gType = t.geometryVector.geometryType(0);
					geomInfo = String(gType);
				} catch (e) {
					geomInfo = "error";
				}
			}
			console.log(`  FeatureTable: ${t.name} | features: ${featureCount} | geometryType(first): ${geomInfo}`);
		}
		return tables;
	} catch (error) {
		console.error(`  ERROR: ${error.message}`);
		return null;
	}
}

export function investigateAllFiles() {
	const files = fs.readdirSync(MLT_DATA_DIR).filter(f => f.endsWith(".mlt"));
	console.log(`Processing ${files.length} MLT files from ${MLT_DATA_DIR}\n`);

	let successCount = 0;
	let errorCount = 0;

	for (const file of files) {
		const filePath = path.join(MLT_DATA_DIR, file);
		const result = investigateEmbeddedMetadata(filePath);
		if (result !== null) {
			successCount++;
		} else {
			errorCount++;
		}
	}

	console.log(`\n========== SUMMARY ==========`);
	console.log(`Total files: ${files.length}`);
	console.log(`Successful: ${successCount}`);
	console.log(`Failed: ${errorCount}`);
}

if (require.main === module) {
	investigateAllFiles();
}
