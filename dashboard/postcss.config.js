const production = !process.env.ROLLUP_WATCH;

module.exports = {
    plugins: [
        require("tailwindcss"),
        require("postcss-import"),
        require('postcss-nested'),
        ...(production
            ? [require("autoprefixer"), require("cssnano")({ preset: "default" })]
            : []),
    ],
};