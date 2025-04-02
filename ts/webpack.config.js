const { CleanWebpackPlugin } = require("clean-webpack-plugin");
const path = require("path");

const ROOT_DIRECTORY = path.resolve("./");
const ENTRY_POINT = path.join(ROOT_DIRECTORY, "src/index.ts");
const DIST_DIRECTORY = path.join(ROOT_DIRECTORY, "dist");

module.exports = {
    context: ROOT_DIRECTORY,
    mode: "production",
    entry: ENTRY_POINT,
    devtool: "source-map",
    output: {
        path: DIST_DIRECTORY,
        filename: "mlt.js",
        library: "mlt",
        libraryTarget: "umd",
        umdNamedDefine: true,
        globalObject: "this",
    },
    module: {
        rules: [
            {
                test: /\.(ts)?$/,
                loader: "ts-loader",
                options: {
                    configFile: "tsconfig.json",
                    silent: true,
                },
                exclude: /node_modules/,
            },
        ],
    },
    resolve: {
        extensions: [".ts", ".js"],
    },
    plugins: [new CleanWebpackPlugin()],
};
