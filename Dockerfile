# Syed Settlers for settlers.pacificfish.ca. GitHub Actions builds this on every push to main
# (.github/workflows/publish.yml); the server only downloads the finished image.
#
# How the server runs it: as user 568 (not root), read-only files, little memory, plain HTTP on PORT
# (3000) behind Cloudflare's HTTPS. The engine is the committed site/js/settlers.wasm, so the image
# needs no Rust.
FROM node:24-alpine

WORKDIR /app
ENV NODE_ENV=production \
    PORT=3000

# Only what runs: the server and the site it serves (no outside packages yet, so no npm install)
COPY server ./server
COPY site ./site

USER node
EXPOSE 3000

# node directly, not "npm start": npm writes logs into a home folder a read-only container lacks
CMD ["node", "server/server.mjs"]
