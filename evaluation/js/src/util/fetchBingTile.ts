/* WARNING: 
   This file is provided for academic purpose only.
   Any other usage is prohibited.
   There is no guarantee that any of the APIs in this file will remain functioning.
 */

import * as MapHelper from "./MapHelper";
import PathNameSpace from "path";
import fs from "fs";

const locationDictionary = {
    seattle: {
        lat: 47.596903,
        lon: -122.379456,
    },
    newyork: {
        lat: 40.71589,
        lon: -74.065106,
    },
    berlin: {
        lat: 52.506126,
        lon: 13.353309,
    },
    london: {
        lat: 51.500212,
        lon: -1.218655,
    },
    nowhere: {
        lat: 19.931847,
        lon: -177.326319,
    },
};

const buildArgsDictionary = (args) => {
    const argsDictioanry = {};
    if (args.length > 3) {
        for (let i = 2; i < args.length; i += 2) {
            const k = args[i];
            const v = args[i + 1];
            argsDictioanry[k] = v;
        }
    }

    return argsDictioanry;
};

function checkFileExistsSync(filepath) {
    let flag = true;
    try {
        fs.accessSync(filepath, fs.constants.F_OK);
    } catch (e) {
        flag = false;
    }
    return flag;
}

const fetchUrlAndSave = (url: string, savePath: string): Promise<string> => {
    const resultPromise: Promise<string> = new Promise<string>((resolve, reject) => {
        fetch(url).then((resp: Response) => {
            if (resp.ok) {
                resp.blob().then((value: Blob) => {
                    value.arrayBuffer().then((arrayBufferValue: ArrayBuffer) => {
                        fs.writeFileSync(savePath, new DataView(arrayBufferValue));
                        resolve(url);
                    });
                });
            } else {
                reject(url);
            }
        });
    });

    return resultPromise;
};

const showUsage = () => {
    console.log("Fetch bing tile util example");
    console.log("npx ts-node fetchBingTile.ts -center newyork -minZoom 10 -maxZoom 10");
};

const currentPath = PathNameSpace.parse(process.argv[1]);
const bingTilePath = PathNameSpace.resolve(currentPath.dir, "../../../../test/fixtures/bing/mvt");
let minZoom = 2;
let maxZoom = 20;

if (process.argv.length < 4) {
    console.log(process.argv);
    showUsage();
} else {
    try {
        const argsDict = buildArgsDictionary(process.argv);
        const centerName = (argsDict["-center"] as string).toLowerCase();
        const center = locationDictionary[centerName];

        const mockScreenWidth = 1024;
        const mockScreenHeight = 768;

        const argMin = argsDict["-minZoom"] as string;
        const argMax = argsDict["-maxZoom"] as string;

        if (argMin) {
            minZoom = parseInt(argMin);
        }

        if (argMax) {
            maxZoom = parseInt(argMax);
        }

        if (maxZoom < minZoom) {
            maxZoom = minZoom;
        }

        if (!center) {
            console.log("Center value must be one of these valuse: ");
            console.log(Object.keys(locationDictionary));
        } else {
            const urlTemplate =
                "https://t.ssl.ak.dynamic.tiles.virtualearth.net/" +
                "comp/ch/{mvtFileName}" +
                "?mkt=en-US&it=G,LC,AP,L,LA&jp=0&js=1&tj=1&ur=us&cstl=s23&mvt=1&features=mvt,mvttxtmaxw,mvtfcall&og=2359&st=bld|v:0_g|pv:1&sv=9.27";

            const allUrls = {};
            for (let z = minZoom; z <= maxZoom; z++) {
                const resultTileSet = MapHelper.getViewTileSets(
                    center.lat,
                    center.lon,
                    z,
                    mockScreenWidth,
                    mockScreenHeight,
                );

                for (const tile of resultTileSet) {
                    const mvtFileName = `${tile.z}-${tile.x}-${tile.y}.mvt`;
                    const url = urlTemplate.replace("{mvtFileName}", mvtFileName);
                    allUrls[mvtFileName] = url;
                }
            }

            const allFiles = Object.keys(allUrls);

            const fetchUrlDriver = (fileIdx: number) => {
                const fileToFetch = allFiles[fileIdx];
                if (fileToFetch) {
                    const fileFullpath = PathNameSpace.join(bingTilePath, fileToFetch);
                    const urlToFetch = allUrls[fileToFetch];

                    if (checkFileExistsSync(fileFullpath)) {
                        console.log(`${fileToFetch} already exists, skipping to next`);
                        fetchUrlDriver(fileIdx + 1);
                    } else {
                        console.log(`fetching ${fileToFetch} ...`);
                        const fetchPromise = fetchUrlAndSave(urlToFetch, fileFullpath);
                        fetchPromise.then(() => {
                            // delay and fetch more
                            setTimeout(() => {
                                fetchUrlDriver(fileIdx + 1);
                            }, 500);
                        });
                    }
                }
            };

            fetchUrlDriver(0);
        }
    } catch (e) {
        console.error(e);
        showUsage();
    }
}
