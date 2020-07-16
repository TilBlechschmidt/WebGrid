FROM node:14.5-alpine AS builder

WORKDIR /app
COPY api/package.json .
COPY api/yarn.lock .

RUN yarn

COPY api/ .
RUN yarn build

# ------
FROM node:14.5-alpine AS api

WORKDIR /app
COPY --from=builder /app/dist/index.js .

CMD ["node", "index.js"]