module.exports = {
    root: true,
    parser: "@typescript-eslint/parser",
    parserOptions: {
        project: ["./tsconfig.eslint.json"],
    },
    rules: {
        "import/no-extraneous-dependencies": ["error", { devDependencies: true }],
        "import/no-mutable-exports": 0,
        "no-labels": 0,
        "no-restricted-syntax": 0,
        semi: [2, "always"],
        "class-methods-use-this": "off",
        "no-underscore-dangle": "off",
        "@typescript-eslint/no-use-before-define": "off",
        "@typescript-eslint/no-namespace": "off",
        "prefer-destructuring": ["error", { object: true, array: false }],
    },
    plugins: ["@typescript-eslint"],
    extends: [
        "airbnb-typescript/base",
        "plugin:@typescript-eslint/recommended",
        "plugin:eslint-comments/recommended",
        "plugin:promise/recommended",
        "prettier",
    ],
};
