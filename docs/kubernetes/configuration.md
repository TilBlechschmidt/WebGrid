# Configuration

To get you started as quickly as possible the helm chart uses a set of default values that work in most clusters. However, these defaults are only an entrypoint — it is very likely that these need to be adapted to your specific needs. The Helm repository contains two charts: `webgrid` and `demo`. The latter wraps the former and provides some additional tools like an instance of [Minio](http://min.io) for storing video recordings and a web UI to access the database.

Other documentation topics may ask you to change values in order to enable advanced features like screen recordings. Refer the sections below to learn how to do so. If you are using the `demo` Chart and want to change values, put them under the `webgrid:` key so they are forwarded to the `webgrid` Chart!

## Changing the defaults

To override values you should create a file called `webgrid-chart-values.yaml` which contains the settings you want to change and apply it using helm:

```bash
# During installation
helm install -f webgrid-chart-values.yaml example webgrid/webgrid

# Change a running grid
helm upgrade -f webgrid-chart-values.yaml example webgrid/webgrid
```

!!! note
    If you changed the name or namespace of the release during installation you have to adjust it accordingly in the example commands above!

## Value reference

Below is a reference of all default helm values with their documentations. You can also find those in the [source code](https://github.com/TilBlechschmidt/WebGrid/blob/main/distribution/kubernetes/chart/values.yaml) of the chart.

<!--codeinclude-->
[Default helm values](../../distribution/kubernetes/demo/charts/webgrid/values.yaml)
<!--/codeinclude-->
