import svelte from 'rollup-plugin-svelte-hot';
import commonjs from '@rollup/plugin-commonjs';
import resolve from '@rollup/plugin-node-resolve';
import livereload from 'rollup-plugin-livereload';
import { terser } from 'rollup-plugin-terser';
import autoPreprocess from 'svelte-preprocess';
import typescript from '@rollup/plugin-typescript';
import replace from '@rollup/plugin-replace';
import child from 'child_process';
import cssHot from 'rollup-plugin-hot-css';
import getConfig from '@roxi/routify/lib/utils/config'
import { copySync, removeSync } from 'fs-extra';
import { spassr } from 'spassr'
import Hmr from 'rollup-plugin-hot'

const production = !process.env.ROLLUP_WATCH;
const isNollup = !!process.env.NOLLUP;
const { distDir } = getConfig() // use Routify's distDir for SSOT
const assetsDir = 'assets'
const buildDir = `${distDir}/build`
const revision = child.execSync('git describe --tags --dirty').toString().trim();

process.env.NODE_ENV = production ? "production" : "development";

// clear previous builds
removeSync(distDir)
removeSync(buildDir)

const serve = () => ({
	writeBundle: async () => {
		const options = {
			assetsDir: [assetsDir, distDir],
			entrypoint: `${assetsDir}/__app.html`,
			script: `${buildDir}/main.js`
		}
		spassr({ ...options, port: 5000 })
	}
})
const copyToDist = () => ({ writeBundle() { copySync(assetsDir, distDir) } })

function generateBasePlugins(compilerOptions) {
	return [
		svelte({
			hot: isNollup,
			preprocess: [
				autoPreprocess({
					sourceMap: !production,
					postcss: require('./postcss.config.js'),
					defaults: { style: 'postcss' }
				})
			],
			compilerOptions: {
				// enable run-time checks when not in production
				dev: !production,
				...compilerOptions
			}
		}),

		// If you have external dependencies installed from
		// npm, you'll most likely need these plugins. In
		// some cases you'll need additional configuration -
		// consult the documentation for details:
		// https://github.com/rollup/plugins/tree/master/packages/commonjs
		resolve({
			browser: true,
			dedupe: importee => !!importee.match(/svelte(\/|$)/)
		}),
		commonjs(),
		typescript({
			sourceMap: !production,
			inlineSources: !production
		}),

		replace({
			preventAssignment: true,
			values: {
				'process.env.NODE_ENV': JSON.stringify(production ? 'production' : 'debug'),
				__buildDate__: () => JSON.stringify(new Date()),
				__buildRevision__: () => JSON.stringify(revision),
				// HMR does not use modules and import.meta is a language syntax feature not an object
				// so to prevent syntax errors we have to replace it.
				'import.meta.url': production ? 'import.meta.url' : '"http://localhost:8080"'
			}
		}),

		production && terser()
	]
}

const dashboardBundle = {
	input: ['src/main.ts'],
	output: {
		sourcemap: !production,
		format: 'es',
		// for performance, disabling filename hashing in development
		chunkFileNames: `[name]${production && '-[hash]' || ''}.js`,
		assetFileNames: `[name][extname]`,
		dir: buildDir
	},
	plugins: [
		// Include all the base plugins
		...generateBasePlugins(),
		cssHot({
			file: 'main.css',
			hot: !production
		}),

		!production && !isNollup && serve(),
		!production && !isNollup && livereload(distDir),
		!production && isNollup && Hmr({ inMemory: true, public: assetsDir }),
		production && copyToDist(),
	],
	watch: {
		clearScreen: false
	}
};

export default dashboardBundle

