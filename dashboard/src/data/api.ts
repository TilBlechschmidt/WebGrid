import { initClient } from '@urql/svelte';
import { setContext } from 'svelte';

export class WebGridAPI {
    static HostKey = 'webgrid-url'

    constructor(host: string | null) {
        // Try guessing the host from the import
        if (host == null) {
            const { origin } = new URL(import.meta.url);
            host = origin;
            console.warn(`No WebGrid host specified. Guessed '${host}' from the import path.`);
        }

        setContext(WebGridAPI.HostKey, host);

        initClient({
            url: `${host}/api`,
        });
    }
}
