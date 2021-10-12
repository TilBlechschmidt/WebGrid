# FAQ

## About this project

### Why does this exist?

As of early 2020 only a handful solutions for distributed selenium based software tests existed. Most of these did not provide support for advanced features like dynamic scaling/provisioning and efficient video recordings with additional problems like thread safety issues and poor scaling for large scale applications with hundreds of browsers.

??? info "Selenium Grid"
    [Selenium Grid](https://www.selenium.dev/documentation/en/grid/) has been in use at my current workplace for some years and worked acceptable. However, with projects growing in both test volume and manpower the amount of concurrent browsers grew rather quickly. This exposed [known bottlenecks](https://github.com/SeleniumHQ/selenium/issues/3574) in the single-proxy design of the Selenium Grid's architecture (which yielded projects like [GridRouter](https://github.com/seleniumkit/gridrouter) which try to work around the fundamental design problem). Additionally, it raised concerns about the constant resource usage due to the static allocation of nodes creating a requirement for dynamic allocation.

    Add to this the fact that Grid 3 has no official support anymore and Grid 4 has been in development for about two years now with no release date in sight (Grid 4 also lacks features like dynamic scalability and screen recordings)!

??? info "Zalenium"
    Zalando ran into similar findings and created an extension to the regular Selenium Grid called [Zalenium](https://github.com/zalando/zalenium). It boasts features like a dashboard with VNC viewers and screen recordings. However, it had its own fair share of issues on top of the ones that the regular grid exposed, yielding even worse test flakiness on a daily basis.

    Zalando stopped maintaining the project as of early 2020.

??? info "Commercial solutions"
    Aerokube provides a commercial off-the-shelve solution for scalable selenium grids in Kubernetes. However, for our application the pricing philosophy was out of reach by a long stretch and it was cheaper to set aside some development resources to create this Open Source solution with the added benefit of making scalable Grids available to the community!

This lead to a investigation of our options to continue Selenium tests with dynamic scaling, screen recording and other future additions on the wish-list. As the underlying protocol of Selenium has been standardized by the W3C we reached the conclusion that it was feasible to develop our own solution to this problem within a few months time. With that this project was born in April 2020.

### Who is behind this?

The project originates from an internal requirement at [PPI AG](https://www.ppi.de/en/). However, as the project progressed it became clear that other people could greatly benefit from it. I have personally taken over the public development and maintenance of the project on GitHub and it is no longer directly affiliated with the PPI AG nor does the company provide any kind of support or responsibility!

I am a software engineer from Germany who recently graduated from the [Nordakademie](https://nordakademie.de) and currently works at PPI AG.

### Why are there so few issues in the tracker?

The project has been developed internally at first using a private GitLab instance. Later, the decision to go public has been made and everything except past issues has been moved over.

## Technical details

### Does it work with Selenium?

Yes it does, thanks to the standardization of the underlying protocol by the W3C. See [below](#what-is-this-sorcery).

### How well does it scale

Let's just say that we haven't reached any performance limits yet in our internal testing with a few hundred browser instances. In theory, the only limit is your cluster bandwidth and to some degree the performance of the Redis database server (although this only affects session creation and not running browser sessions).

### What is this sorcery?! 🧙‍♂️

No magic, just standardization and an efficient architecture 😉

WebGrid relies on the [WebDriver](https://www.w3.org/TR/webdriver1/) specification. When you execute a Selenium Test, you are effectively speaking to an implementation of this protocol. Almost all browser vendors provide such an API for their browser. Both Selenium Grid and WebGrid are just intermediates who delegate your requests to a browser.

The standardization allows us to implement a completely transparent solution which looks just like your local browser or Selenium Grid. Furthermore, by employing a modern architecture, which is built for stability and performance with clusters and dynamic deployment of browsers in mind, we can provide significantly better scalability and speed!

If you have any questions about the inner workings don't hesitate to ask either on [GitHub Discussions](https://github.com/TilBlechschmidt/WebGrid/discussions), the [official Discord server](https://discord.gg/yYaPcNM), or by contacting me via [mail](mailto:til@blechschmidt.dev)!

### Where is the latest tag?

We believe that the `:latest` in Docker [is](https://vsupalov.com/docker-latest-tag/) [very](https://medium.com/@mccode/the-misunderstood-docker-tag-latest-af3babfd6375) [evil](https://blog.container-solutions.com/docker-latest-confusion) [and](https://developers.redhat.com/blog/2016/02/24/10-things-to-avoid-in-docker-containers/) [dangerous](https://medium.com/@tariq.m.islam/container-deployments-a-lesson-in-deterministic-ops-a4a467b14a03). Apart from the problem that it is just *yet another tag* and everything else being purely convention, you should make a conscious choice to upgrade your production environment from one version to another.

For this reason we are releasing versions on all distribution platforms following the [SemVer 2.0](https://semver.org) convention so you can know which versions are safe and which might require some more work. Additionally, all Helm charts and Compose files use pinned versions for the Docker images.