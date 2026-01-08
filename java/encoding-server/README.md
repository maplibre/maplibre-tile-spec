# MVT to MLT Development Server

This Node.js-based application serves as a **development and testing server** for converting **MVT** to **MLT** in realtime.
The server acts as a proxy for style, source, and tile endpoints, enabling seamless on-demand transformation of vector tile data.

> Most of the arguments supported by the java encoder are also available as Node.js arguments or through `config.json` file.

---

## Prerequisites

- Node.js
- npm (Node package manager)
- Java MLT encoder

## Running the Server

```bash
cd encoding-server
npm install
npm start
```

## Requests

In most cases, the initial style request is sufficient to initiate the process, with all subsequent source and tile requests automatically redirected back to the server.

```bash
## style - http://<server_ip>/style?url=<style_url>
curl http://0.0.0.0/style?url=https://demotiles.maplibre.org/style.json ## default
curl http://localhost/style?url=https://demotiles.maplibre.org/style.json
curl http://10.0.2.2/style?url=https://demotiles.maplibre.org/style.json ## Android emulator bridge to 0.0.0.0

## source - http://<server_ip>/style?url=<source_url>
curl http://0.0.0.0/source?url=https://demotiles.maplibre.org/tiles/tiles.json

## tile - http://<server_ip>/tile?url=<tile_url>
curl http://0.0.0.0/tile?url=https://demotiles.maplibre.org/tiles/{z}/{x}/{y}.pbf

```
