import * as MapHelper from "./MapHelper";
import PathNameSpace from "path";
import fs from "fs";

const showUsage = () => {
    console.log("Notice all '&' chars in URL need to be escaped.");
    console.log("For example:\r\n");
    console.log(
        "npx ts-node fetchMVTTile.ts -center newyork -minZoom 10 -maxZoom 10 -urlTemplate https://foo.com/{x}-{y}-{z}?parameter1=p1^parameter2=p2",
    );
};

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

console.log(process.argv);
const currentPath = PathNameSpace.parse(process.argv[1]);
const bingTilePath = PathNameSpace.resolve(currentPath.dir, "../../../../test/fixtures/bing/mvt");
let minZoom = 2;
let maxZoom = 20;

if (process.argv.length < 6) {
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

        const urlTemplate = argsDict["-urlTemplate"] as string;

        if (argMin) {
            minZoom = parseInt(argMin);
        }

        if (argMax) {
            maxZoom = parseInt(argMax);
        }

        if (maxZoom < minZoom) {
            maxZoom = minZoom;
        }

        const urlTemplateValid =
            urlTemplate &&
            urlTemplate.indexOf("{x}") > -1 &&
            urlTemplate.indexOf("{y}") > -1 &&
            urlTemplate.indexOf("{z}") > -1;

        if (!center || !urlTemplateValid) {
            showUsage();
        } else {
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
                    const url = urlTemplate
                        .replace("{x}", tile.x.toString())
                        .replace("{y}", tile.y.toString())
                        .replace("{z}", tile.z.toString());
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
