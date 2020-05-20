# Quick-Start Guide
The project is still very much in flux and thus a detailed user-facing documentation does not make much sense. However, you may grab the Helm Chart in this repo and deploy it :)

```
git clone <repo-url> && cd <repo-name>
helm install test ./web-grid
```

Have a look at the values.yaml and adapt it to your environment.

All images are currently served from a rate-limited private registry that is only meant for development use!