FROM alpine

RUN apk add --no-cache python3 py3-pip git curl bash

RUN pip3 install --no-cache \
            'mkdocs-git-revision-date-localized-plugin>=0.4' \
            'mkdocs-material' \
            'mkdocs-mermaid2-plugin' \
            'mkdocs-codeinclude-plugin' \
            'mkdocs-material-extensions' \
            'mkdocs-simple-hooks' \
            'git+http://github.com/TilBlechschmidt/mkdocs-helm'

RUN curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | VERIFY_CHECKSUM=false bash

WORKDIR /docs
CMD mkdocs serve -a 0.0.0.0:8000
