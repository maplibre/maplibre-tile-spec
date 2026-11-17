import { createReadStream } from "node:fs";
import { stat } from "node:fs/promises";
import { createServer } from "node:http";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
// Serve from the repository root so `/js/viewer/meshes.html` can also reach
// `/rust/*.pmtiles` and `/test/fixtures/meshes/tiles` via `?url=` or `?tiles=`.
const ROOT = path.resolve(__dirname, "../..");
const PORT = Number(process.env.PORT ?? 8080);

const MIME_TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".mlt": "application/octet-stream",
  ".pmtiles": "application/octet-stream",
  ".wasm": "application/wasm",
};

const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "localhost"}`);
    if (url.pathname === "/" || url.pathname === "/js" || url.pathname === "/js/") {
      res.writeHead(302, { Location: `/js/viewer/meshes.html${url.search}` });
      res.end();
      return;
    }

    const decodedPath = decodeURIComponent(url.pathname).replace(/^\/+/, "");
    if (
      (decodedPath === "js/dist/columns.js" || decodedPath === "js/dist/index.js") &&
      !(await stat(path.resolve(ROOT, decodedPath)).catch(() => null))
    ) {
      res.writeHead(302, { Location: `/${decodedPath.replace("js/dist/", "js/src/")}` });
      res.end();
      return;
    }
    const filePath = path.resolve(ROOT, decodedPath);
    const rel = path.relative(ROOT, filePath);
    if (rel.startsWith("..") || path.isAbsolute(rel)) {
      res.writeHead(403);
      res.end("Forbidden");
      return;
    }

    const info = await stat(filePath);
    if (!info.isFile()) {
      res.writeHead(404);
      res.end("Not found");
      return;
    }

    const ext = path.extname(filePath).toLowerCase();
    const contentType = MIME_TYPES[ext] ?? "application/octet-stream";
    const range = req.headers.range;

    if (range) {
      const match = /^bytes=(\d+)-(\d*)$/.exec(range);
      if (!match) {
        res.writeHead(416, { "Content-Range": `bytes */${info.size}` });
        res.end();
        return;
      }
      const start = Number(match[1]);
      const end = match[2] ? Math.min(Number(match[2]), info.size - 1) : info.size - 1;
      if (start >= info.size || start > end) {
        res.writeHead(416, { "Content-Range": `bytes */${info.size}` });
        res.end();
        return;
      }
      res.writeHead(206, {
        "Content-Type": contentType,
        "Accept-Ranges": "bytes",
        "Content-Range": `bytes ${start}-${end}/${info.size}`,
        "Content-Length": end - start + 1,
      });
      if (req.method === "HEAD") {
        res.end();
        return;
      }
      createReadStream(filePath, { start, end }).pipe(res);
      return;
    }

    res.writeHead(200, {
      "Content-Type": contentType,
      "Accept-Ranges": "bytes",
      "Content-Length": info.size,
    });
    if (req.method === "HEAD") {
      res.end();
      return;
    }
    createReadStream(filePath).pipe(res);
  } catch {
    res.writeHead(404);
    res.end("Not found");
  }
});

server.listen(PORT, "127.0.0.1", () => {
  console.log(`MLT JS viewer running at:`);
  console.log(`  http://localhost:${PORT}/js/viewer/meshes.html`);
  console.log(
    `  http://localhost:${PORT}/js/viewer/meshes.html?url=/rust/hamburg_v3_lod_v7.pmtiles`,
  );
});
