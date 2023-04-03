import { Database, OPEN_READONLY } from "sqlite3";
import { promisify } from "util";
import {BoundingBox} from "./geometry";

export interface Metadata{
    name: string;
    boundingBox: BoundingBox;
    minZoom: number;
    maxZoom: number;
    format: string;
    attribution: string;
    layers: string;
}

export default class MbTilesRepository {
    private static readonly METADATA_TABLE_NAME = "metadata";
    private static readonly TILES_TABLE_NAME = "tiles";

    private constructor(private readonly db: Database) {}

    static async create(fileName: string): Promise<MbTilesRepository> {
        const db = await MbTilesRepository.connect(fileName);
        return new MbTilesRepository(db);
    }

    async getMetadata(): Promise<Partial<Metadata>> {
        const query = `SELECT name, value FROM ${MbTilesRepository.METADATA_TABLE_NAME};`;
        const rows = await promisify(this.db.all.bind(this.db))(query);

        const metadata: Partial<Metadata> = {}
        for (const row of rows) {
            switch (row.name) {
                case "name":
                    Object.assign(metadata, {name: row.value});
                    break;
                case "bounds": {
                    const boundingBox = row.value.split(",").map((value) => parseFloat(value.trim()));
                    Object.assign(metadata, {boundingBox: boundingBox});
                    break;
                }
                case "minzoom":
                    Object.assign(metadata, {minZoom: row.value})
                    break;
                case "maxzoom":
                    Object.assign(metadata, {maxZoom: row.value})
                    break;
                case "format":
                    Object.assign(metadata, {tileFormat: row.value})
                    break;
                case "attribution":
                    Object.assign(metadata, {attribution: row.value})
                    break;
                case "json":
                    Object.assign(metadata, {layers: row.value})
                    break;
            }
        }

        return metadata;
    }

    async getTile(xyzTileIndex: {x: number, y: number, z: number}): Promise<Uint8Array> {
        const tmsY = 2 ** xyzTileIndex.z - xyzTileIndex.y - 1;
        const tmsIndex = { ...xyzTileIndex, y: tmsY };

        return new Promise((resolve, reject) => {
            this.db.get(
                `SELECT tile_data from ${MbTilesRepository.TILES_TABLE_NAME} where zoom_level=? AND tile_column=? AND tile_row=?`,
                [tmsIndex.z, tmsIndex.x, tmsIndex.y],
                (err, row) => {
                    if (err) {
                        reject();
                        return;
                    }

                    if(!row){
                        //TODO: remove -> quick and dirty approach for out of bounds problem
                        resolve(null);
                        return;
                    }

                    resolve(row.tile_data);
                },
            );
        });
    }

    async dispose(): Promise<void> {
        return promisify(this.db.close.bind(this.db))();
    }

    private static connect(dbPath: string): Promise<Database> {
        return new Promise<Database>((resolve, reject) => {
            const db = new Database(dbPath, OPEN_READONLY, (err) => {
                if (err) {
                    reject(err.message);
                    return;
                }

                resolve(db);
            });
        });
    }
}



