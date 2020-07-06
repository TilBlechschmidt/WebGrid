# Accessing the grid

By default the chart does not create an Ingress object for the Service. This means that the grid will only be accessible from within your cluster. You can use any Kubernetes mechanic to expose it to your clients — below are some examples to point you in the right direction.

!!! note
    If you changed the name or namespace of the release during installation you have to adjust it accordingly in the examples below!

## Cluster internal access

If your Selenium clients (e.g. test suites) are running inside your cluster you can directly interact with the Service created by the helm chart. In the default Kubernetes setup you can access it through either `http://example-webgrid` within the same namespace or `http://example-webgrid.namespace` from a different one.

## Port forwarding

In case you have a local client and want to use the grid you can temporarily forward a local port to the Service in the cluster. Below is an example that would expose the grid at `http://localhost:3030`.

```
kubectl port-forward service/example-web-grid 3030:80
```

## Ingress object

If you want to access the grid from outside the cluster you can use any method Kubernetes provides to expose the Service. Below is an example configuration object.

```yaml
apiVersion: networking.k8s.io/v1beta1
kind: Ingress
metadata:
  name: webgrid
spec:
  rules:
    - host: webgrid.your-host.dev
      http:
        paths:
          - path: /
            backend:
              serviceName: example-web-grid
              servicePort: http
```

!!! note
    You may have to add additional properties to the spec for it to work depending on your cluster setup. Consult your cluster admin (or documentation) for more details.

