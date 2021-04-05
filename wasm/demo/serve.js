const {buildOption} = require('./build.js')

require('esbuild').serve({
    servedir: "."
}, buildOption).then(serveResult => {
    console.log(`Listening http://${serveResult.host}:${serveResult.port}`);
}).catch(() => process.exit(1))
