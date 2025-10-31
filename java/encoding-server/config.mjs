import { parseArgs } from "node:util";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const configFile = JSON.parse(
  fs.readFileSync(path.join(__dirname, "config.json")),
);

const { values: args } = parseArgs({
  options: {
    host: { type: "string", default: "0.0.0.0" },
    port: { type: "string", default: "80" },
    verbose: { type: "boolean", default: true },
    keep_files: { type: "boolean", default: false },
    noencodingserver: { type: "boolean", default: false },
    input: { type: "string", default: "mvt" },
    noids: { type: "boolean", default: false },
    advanced: { type: "boolean", default: false },
    nomorton: { type: "boolean", default: false },
    outlines: { type: "string", default: "" },
    timer: { type: "boolean", default: false },
    compare: { type: "boolean", default: false },
  },
  args: process.argv.slice(2),
});

const config = {
  ...args,
  ...configFile,
  port: Number(configFile.port ?? args.port ?? 80),
};

config.cachePath = path.join(__dirname, "cache");
config.cliToolsPath = path.join(__dirname, "../");
config.encoderPath = path.join(
  config.cliToolsPath,
  "mlt-cli/build/libs/encode.jar",
);
config.encoderPort = 3001;

export default config;
