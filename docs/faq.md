# FAQ

## Technical details

### Does it work with Selenium?

Yes it does, thanks to the standardization of the underlying protocol by the W3C.

### How well does it scale

Let's just say that we haven't reached any performance limits yet in our internal testing with a few hundred browser instances. In theory, the only limit is your cluster bandwidth and to some degree the performance of the Redis database server (although this only affects session creation and not running browser sessions).

## About this project

### Why does this exist?

As of early 2020 only a handful solutions for distributed selenium based software tests existed. Most of these did not provide support for advanced features like dynamic scaling/provisioning and efficient video recordings with additional problems like thread safety issues and poor scaling for large scale applications with hundreds of browsers.

??? info "Selenium Grid"
    [Selenium Grid](https://www.selenium.dev/documentation/en/grid/) has been in use at my current workplace for some years and worked acceptable. However, with projects growing in both test volume and manpower the amount of concurrent browsers grew rather quickly. This exposed [known bottlenecks](https://github.com/SeleniumHQ/selenium/issues/3574) in the single-proxy design of the Selenium Grid's architecture (which yielded projects like [GridRouter](https://github.com/seleniumkit/gridrouter) which try to work around the fundamental design problem). Additionally, it raised concerns about the constant resource usage due to the static allocation of nodes creating a requirement for dynamic allocation.

    Add to this the fact that Grid 3 has no official support anymore and Grid 4 has been in development for about two years now with no release date in sight!

??? info "Zalenium"
    Zalando ran into similar findings and created an extension to the regular Selenium Grid called [Zalenium](https://github.com/zalando/zalenium). It boasts features like a dashboard with VNC viewers and screen recordings. However, it had some its own fair share of issues on top of the ones that the regular grid exposed, yielding even worse test flakiness on a daily basis.

    Zalando stopped maintaining the project as of early 2020.

??? info "Commercial solutions"
    Aerokube provides a commercial off-the-shelve solution for scalable selenium grids in Kubernetes. However, for our application the pricing philosophy was out of reach by a long stretch and it was cheaper to set aside some development resources to create this Open Source solution with the added benefit of making scalable Grids available to the community!

This lead to a investigation of our options to continue Selenium tests with dynamic scaling, screen recording and other future additions on the wish-list. As the underlying protocol of Selenium has been standardized by the W3C we reached the conclusion that it was feasible to develop our own solution to this problem within a few months time. With that this project was born in April 2020.

### Who is behind this?

The project originates from an internal requirement at [PPI AG](https://www.ppi.de/en/). However, as the project progressed it became clear that other people could greatly benefit from it. I have personally taken over the public development and maintenance of the project on GitHub and it is no longer directly affiliated with the PPI AG nor does the company provide any kind of support or responsibility!

I am a computer science student from Germany, currently working at PPI AG and studying Applied Computer Science at the [Nordakademie](https://nordakademie.de) in my last semester.

### Why are there so few issues in the tracker?

The project has been developed internally at first using a private GitLab instance. Later, the decision to go public has been made and everything except past issues has been moved over.