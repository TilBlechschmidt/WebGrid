# Deploying in K3s

This guide will explain how to get a basic but feature-complete instance of WebGrid up and running in a cluster using [k3s by Rancher](https://k3s.io). It will assume that you have a basic k3s cluster running and a working `kubectl` and `helm` locally available.

## Installing with Helm

To get started quickly, we will be deploying the `demo` chart. It contains the core grid along with external services that are required to get additional features like video recordings running.

```bash
# Add the repository
helm repo add webgrid https://webgrid.dev/

# List all available versions
helm search repo --versions --devel webgrid/demo

# Install the chart
helm install k3s-guide-deployment webgrid/demo --version "<pick-a-version-from-the-list>"
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
    ghcr.io/tilblechschmidt/parallelseleniumtest:sha-fa30ad9
```

After experimenting a bit with it, you can visit your grid in a browser — just put the same URL in that you use for your Selenium clients — to view the screen recordings for your sessions!