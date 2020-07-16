const fs = require('fs')

const hlsjs = fs.readFileSync('node_modules/hls.js/dist/hls.min.js', 'utf8')
let embed = fs.readFileSync('resources/embed.js', 'utf8')

embed = embed.replace('// {{Hls.js-embed}}', hlsjs)

fs.writeFileSync('src/generated.ts', `/* eslint-disable-next-line quotes */\nexport const embed = ${JSON.stringify(embed)}`)