export default {
    platformName: ({ _orchestratorID }, _, { redis }) => redis.get(`orchestrator:${_orchestratorID}:capabilities:platformName`),
    browsers: ({ _orchestratorID }, _, { redis }) =>
        redis.smembers(`orchestrator:${_orchestratorID}:capabilities:browsers`)
            .then(browsers => browsers.map(browser => {
                const components = browser.split('::')

                return {
                    name: components[0],
                    version: components[1]
                }
            }))
}