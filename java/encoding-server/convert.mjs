import { unlink, existsSync, mkdirSync, createWriteStream } from "node:fs";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
import { join } from "node:path";
import { randomUUID } from "node:crypto";
import { exec, execSync, spawn } from "node:child_process";
import net from "node:net";

import config from "./config.mjs";

function convertRequest(convertResponse) {
  return (req, res) => {
    if (config.verbose) {
      console.log(req.originalUrl);
    }

    if (!req.query.url) {
      if (config.verbose) {
        console.error("Missing `url` parameter");
      }

      res.status(400).send("Missing `url` parameter");
      return;
    }

    let url;
    try {
      url = new URL(req.query.url);
    } catch {
      if (config.verbose) {
        console.error(`Invalid url ${req.query.url}`);
      }

      res.status(400).send(`Invalid url: ${req.query.url}`);
      return;
    }

    fetch(url)
      .then((styleResponse) => styleResponse.text())
      .then((data) => convertResponse(req, data, res))
      .catch((error) => {
        if (config.verbose) {
          console.error(`Request failed: ${req.query.url} - ${error}`);
        }
        res.status(500).send(`Request failed: ${req.query.url} - ${error}`);
      });
  };
}

function convertURL(urlString, type, req) {
  try {
    const url = new URL(urlString);

    if (url.protocol != "https:" && url.protocol != "http:") {
      return urlString;
    }
  } catch (error) {
    if (config.verbose) {
      console.error(`URL (${urlString}) parse error: ${error}`);
    }
    return urlString;
  }

  return `${req.protocol}://${req.get("host")}/${type}?url=${urlString}`;
}

function convertStyleResponse(req, data, res) {
  try {
    const json = JSON.parse(data);

    if (json.sources) {
      for (let key in json.sources) {
        if (!Object.hasOwn(json.sources, key)) {
          continue;
        }

        var source = json.sources[key];

        if (!source || source.type != "vector") {
          continue;
        }

        source.encoding = "mlt";

        if (source.url) {
          source.url = convertURL(source.url, "source", req);
        }

        if (source.tiles) {
          source.tiles = source.tiles.map((tile) => {
            return convertURL(tile, "tile", req);
          });
        }
      }
    }

    res.status(200).json(json);
  } catch (error) {
    if (config.verbose) {
      console.error(`Failed to parse style response: ${error}`);
    }
    res.status(400).send(`Failed to parse style response: ${error}`);
  }
}

function convertSourceResponse(req, data, res) {
  try {
    const json = JSON.parse(data);

    json.encoding = "mlt";

    if (json.tiles) {
      for (let key in json.tiles) {
        if (!Object.hasOwn(json.tiles, key)) {
          continue;
        }

        json.tiles[key] = convertURL(json.tiles[key], "tile", req);
      }
    }

    res.status(200).json(json);
  } catch (error) {
    if (config.verbose) {
      console.error(`Failed to parse style response: ${error}`);
    }
    res.status(400).send(`Failed to parse style response: ${error}`);
  }
}

const convertStyleRequest = convertRequest(convertStyleResponse);
const convertSourceRequest = convertRequest(convertSourceResponse);

function convertTileResponse(filePath, res) {
  const mltPath = filePath + ".mlt";
  const args =
    " --" +
    config.input +
    " " +
    filePath +
    " --mlt " +
    mltPath +
    (config.noids ? " --noids" : "") +
    (config.fsst ? " --fsst" : "") +
    (config.fastpfor ? " --fastpfor" : "") +
    (config.nomorton ? " --nomorton" : "") +
    (config.outlines ? " --outlines " + config.outlines : "") +
    (config.tessellate ? " --tessellate" : "") +
    (config.coercemismatch ? " --coerce-mismatch" : "") +
    (config.timer ? " --timer" : "") +
    (config.compare ? " --compare-all" : "");

  const callback = (error, stdout, stderr) => {
    if (config.verbose) {
      if (stdout) {
        console.log(`Encoder output: ${stderr}`);
      }

      if (stderr) {
        console.error(`Encoder error: ${stderr}`);
      }
    }

    if (!config.keep_files) {
      unlink(filePath, (fileErr) => {
        if (fileErr && config.verbose) {
          console.error(
            `Failed to delete input file: ${filePath} - ${fileErr}`,
          );
        }
      });
    }

    if (error) {
      if (config.verbose) {
        console.error(`Tile encoding failed: ${error}`);
      }

      res.status(500).send(`Tile encoding failed: ${error}`);
      return;
    }

    res.on("finish", () => {
      if (!config.keep_files) {
        unlink(mltPath, (fileErr) => {
          if (fileErr && config.verbose) {
            console.error(
              `Failed to delete output file: ${mltPath} - ${fileErr}`,
            );
          }
        });
      }
    });

    res.status(200).sendFile(mltPath);
  };

  if (config.noencodingserver) {
    convertTileCLI(args, callback);
  } else {
    convertTileCLIServer(args, callback);
  }
}

function convertTileCLI(args, callback) {
  const command = `java -jar ${config.encoderPath} ${args}`;
  exec(command, callback);
}

function convertTileCLIServer(args, callback) {
  const command = `${args}\n`;
  let response = "";
  const socket = new net.Socket();

  socket.connect(config.encoderPort, "localhost", () => {
    socket.write(command);
  });

  socket.on("data", (data) => {
    response += data;
  });

  socket.on("close", () => {
    callback(null, response.length > 0 ? response : null, null);
  });

  socket.on("error", (error) => {
    console.error(`Encoder error: ${error}`);
  });
}

function convertTileRequest(req, res) {
  if (config.verbose) {
    console.log(req.originalUrl);
  }

  if (!req.query.url) {
    if (config.verbose) {
      console.error("Missing `url` parameter");
    }

    res.status(400).send("Missing `url` parameter");
    return;
  }

  let url;
  try {
    url = new URL(req.query.url);
  } catch {
    if (config.verbose) {
      console.error(`Invalid url: ${req.query.url}`);
    }

    res.status(400).send(`Invalid url: ${req.query.url}`);
    return;
  }

  fetch(url)
    .then(async (tileResponse) => {
      if (!tileResponse.ok) {
        res
          .status(tileResponse.status)
          .send(
            `Tile request error: ${req.query.url} - ${tileResponse.status} ${tileResponse.statusText}`,
          );
        return;
      }

      if (!existsSync(config.cachePath)) {
        mkdirSync(config.cachePath, { recursive: true });
      }

      const file = createWriteStream(join(config.cachePath, randomUUID()));

      try {
        await pipeline(Readable.fromWeb(tileResponse.body), file);
      } catch (error) {
        if (config.verbose) {
          console.error(`Tile download failed: ${req.query.url} - ${error}`);
        }
        res.status(500).send(`Tile download failed: ${req.query.url} - ${error}`);
        return;
      }

      convertTileResponse(file.path, res);
    })
    .catch((error) => {
      if (config.verbose) {
        console.error(`Request failed: ${req.query.url} - ${error}`);
      }
      res.status(500).send(`Request failed: ${req.query.url} - ${error}`);
    });
}

function runCLISetup() {
  console.log(`Building CLI tools at ${config.cliToolsPath}`);
  execSync(`./gradlew cli`, { cwd: config.cliToolsPath });

  if (config.noencodingserver) {
    return;
  }

  const server = spawn(`java`, [
    "-jar",
    `${config.encoderPath}`,
    "--server",
    `${config.encoderPort}`,
  ]);

  if (config.verbose) {
    server.stdout.on("data", (data) => {
      console.log(`Encoder: ${data}`);
    });

    server.stderr.on("data", (data) => {
      console.error(`Encoder: ${data}`);
    });
  }

  server.on("close", (code) => {
    console.log(`Encoder closed: ${code}`);
  });
}

export {
  convertStyleRequest,
  convertSourceRequest,
  convertTileRequest,
  runCLISetup,
};
