module.exports = {
    displayName: 'Unit',
    transform: {
        "^.+\\.ts": "ts-jest",
    },
    testRegex: "(.*|(\\.|/)(test|spec))\\.(js|ts)$",
    moduleFileExtensions: ["ts", "js", "json", "node"],
};
