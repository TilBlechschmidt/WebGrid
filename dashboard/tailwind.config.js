const production = !process.env.ROLLUP_WATCH;

module.exports = {
  darkMode: false, // or 'media' or 'class'
  important: '.webgrid',
  theme: {
    extend: {}
  },
  variants: {
    extend: {},
  },
  plugins: [
  ],
  future: {
    purgeLayersByDefault: true,
    removeDeprecatedGapUtilities: true,
  },
  purge: {
    content: [
      "./src/**/*.svelte",
    ],
    enabled: production // disable purge in dev
  },
}
