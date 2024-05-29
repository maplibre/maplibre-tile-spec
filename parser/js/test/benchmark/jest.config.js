module.exports = {
    displayName: 'Benchmarking',
    transform: {
        "^.+\\.ts": "ts-jest",
    },
    testRegex: "(/test/benchmarking/.*|(\\.|/)(test|spec))\\.(js|ts)$",
    moduleFileExtensions: ["ts", "js", "json", "node"],
};
