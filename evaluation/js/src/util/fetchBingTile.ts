console.log("Fetch bing tile util");
import * as MapHelper from "./MapHelper";

const locationDictionary = {
    seattle: {
        lat: 47.596903,
        lon: -122.379456,
    },
    newyork: {
        lat: 40.71589,
        lon: -74.065106,
    }
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

console.log(buildArgsDictionary(process.argv));

const resultTileSet = MapHelper.getViewTileSets(
    locationDictionary["newyork"].lat,
    locationDictionary["newyork"].lon,
    8,
    1024,
    768
);
console.log(resultTileSet);
