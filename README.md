# WebGrid
<p align="center">
  <img width="75%" src="http://placekitten.com/882/250" alt="Banner">
</p>
<p align="center">
  <b>Scalable, fast, resilient WebDriver network.</b>
</p>

## About
WebGrid is a combination of services that provide a WebDriver API for use with Selenium Clients. It has the capability to dynamically scale as demand changes and can support an unlimited number of Browsers in one network[^Upper limit has not yet been tested/reached ;)]! Its components have been designed to be highly resilient and allow fast recovery from any errors or outages, ensuring stability of your automation tasks.

At its core WebGrid is a stateful multiplexing proxy that starts Browsers on demand, however it has been split into independent and fully scalable components which allows to increase the scale if any bottlenecks are encountered (including the ingress proxy)!

## Use cases
WebGrid is aiming to provide a scalable alternative to the popular Selenium Grid. This has been made possible by the standardisation of the [WebDriver protocol](https://w3c.github.io/webdriver/) by the W3C which is the foundation of Selenium. It contains remote control methods for browsers and has been adopted by all major vendors. Additionally it incorporates support for a stateful intermediate proxy to distribute load between multiple machines.

Thanks to its independent components WebGrid is perfect for anybody that has an existing cluster infrastructure and wants to run a dynamically scalable browser automation network on it for e.g. UI tests.

## Requirements
The grid requires an infrastructure to provision new Browser hosts. This can range from a single PC with its existing operating system and browser or a local Docker all the way to a multi-site K8s cluster.

Since the project is currently undergoing active development you have to clone the repository to make use of it. The current setup only supports local Docker as a provisioner which will change in the future. To build all the required images and start a fully operational grid you can execute the following commands:

```bash
make images
make run
```

You can then send requests to `http://localhost/`. Screen recordings are currently stored in `/tmp/vr` named by the session identifiers.

For more details on how the systems work, take a look at the documentation directory.