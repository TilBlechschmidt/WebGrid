const embedScriptTag = document.currentScript

// {{Hls.js-embed}}

function fetchVideoURL(sessionID, host) {
    return new Promise((resolve, reject) => {
        const storageQuery = `query {
            session(id: "${sessionID}") {
                videoURL
            }
        }`

        let xhr = new XMLHttpRequest()
        const cleanedHost = host.substring(0, host.lastIndexOf('/'))
        xhr.open('POST', `${cleanedHost}/api`, true)
        xhr.setRequestHeader('Content-type', 'application/json')
        xhr.onload = function () {
            try {
                const response = JSON.parse(this.responseText)
                resolve(response.data.session.videoURL)
            } catch (e) {
                reject(e)
            }
        }
        xhr.send(JSON.stringify({ query: storageQuery }))
    })
}

class VideoElement extends HTMLElement {
    static get observedAttributes() { return ['sessionID', 'host'] }

    constructor() {
        super()

        this.root = this.attachShadow({ mode: 'closed' })

        const style = document.createElement('style')
        style.textContent = `
            .container {
                background-color: green;
                position: relative;
                width: 100%;
                padding-top: 56.25%;
            }

            video {
                background-color: #d5e1df;
                position: absolute;
                top: 0;
                left: 0;
                bottom: 0;
                right: 0;
                width: 100%;
            }
        `

        const container = document.createElement('div')
        container.className = 'container'

        const video = document.createElement('video')
        video.id = 'webgrid-video'
        video.controls = true
        container.appendChild(video)

        this.root.appendChild(style)
        this.root.appendChild(container)
    }

    connectedCallback() {
        this.loadMedia()
    }

    // Omitted parameters: name, oldValue, newValue
    attributeChangedCallback() {
        this.loadMedia()
    }

    loadMedia() {
        const scriptSrc = embedScriptTag.src

        const sessionID = this.getAttribute('session-id')
        const host = this.getAttribute('host') || scriptSrc
        const video = this.root.getElementById('webgrid-video')

        fetchVideoURL(sessionID, host).then(videoURL => {
            // eslint-disable-next-line no-undef
            const hls = new Hls()

            console.log(videoURL)

            hls.loadSource(videoURL)
            hls.attachMedia(video)
        })
    }
}

customElements.define('webgrid-video', VideoElement)
