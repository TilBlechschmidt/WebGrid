# Deploying in K3s

This guide will explain how to get a basic but feature-complete instance of WebGrid up and running in a cluster using [k3s by Rancher](https://k3s.io). It will assume that you have a basic k3s cluster running and a working `kubectl` and `helm` locally available.

## Installing with Helm

Since video recording is disabled by default, we have to set a config value to enable it. For this guide it is not required to modify any other variables as the defaults are suited for K3s. However, this may differ if you deploy to a non-K3s cluster. Consult the [storage documentation](../kubernetes/storage.md) for more details.

To set the value, we will use the `--set` command line option but a values file would work the same way.

```bash
# Add the repository
helm repo add webgrid https://webgrid.dev/

# Install the chart
helm install k3s-guide-deployment webgrid/webgrid --set recording.enabled=true
```

## Gaining access

For the simplicity of this guide, we will use a `NodePort` service to access the grid. To do so, create a file named `webgrid-nodeport.yml` with the following content:

```yaml
apiVersion: v1
kind: Service
metadata:
  name: guide-webgrid-nodeport
spec:
  type: NodePort
  ports:
    - port: 80
      targetPort: http
      nodePort: 30007
      protocol: TCP
      name: http
  selector:
    web-grid/component: proxy
    app.kubernetes.io/name: webgrid
    app.kubernetes.io/instance: k3s-guide-deployment
```

You can now apply it by using the following command:

```bash
kubectl apply -f webgrid-nodeport.yml
```

## Giving it a spin

The grid is now fully operational and available on port `30007` of your cluster! You can either point your existing Selenium tests at it or use our testing tool by running the following command which runs three tests locally in Docker (you have to insert the clusters address):

```bash
docker run --rm -it \
    -e ENDPOINT=http://<your-grid-endpoint>:30007 \
    -e FORKS=3 \
    ghcr.io/tilblechschmidt/parallelseleniumtest:sha-14540b77
```

After experimenting a bit with it, you can visit your grid in a browser — just put the same URL in that you use for your Selenium clients — to view the screen recordings for your sessions!