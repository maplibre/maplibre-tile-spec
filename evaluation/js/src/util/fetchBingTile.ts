import * as MapHelper from "./MapHelper";
import path from "path";

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

const currentPath = path.parse(process.argv[1]);
const bingTilePath = path.resolve(currentPath.dir, "../../../../test/fixtures/bing/mvt");

if (process.argv.length < 4) {
    console.log(process.argv);
    console.log("Fetch bing tile util example");
    console.log("npx ts-node src/util/fetchBingTile.ts -center newyork");
} else {
    const argsDict = buildArgsDictionary(process.argv);
    const centerName = (argsDict["-center"] as string).toLowerCase();
    const center = locationDictionary[centerName];

    const mockScreenWidth = 1024;
    const mockScreenHeight = 768;

    if (!center) {
        console.log("Center value must be one of these valuse: ");
        console.log(Object.keys(locationDictionary));
    } else {
        for (let z = 2; z < 20; z++) {
            const resultTileSet = MapHelper.getViewTileSets(
                center.lat,
                center.lon,
                z,
                mockScreenWidth,
                mockScreenHeight,
            );

            console.log(resultTileSet);
        }
    }
}
