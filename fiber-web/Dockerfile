FROM node:20-slim

RUN apt-get update && apt-get install -y git && rm -rf /var/lib/apt/lists/*

RUN cd /opt && git clone --depth 1 https://github.com/gpBlockchain/fiber-nodes-monit.git

WORKDIR /opt/fiber-nodes-monit

RUN npm ci && npm run build

ENV HOST=0.0.0.0
ENV PORT=3000

EXPOSE 3000

CMD ["npm", "start"]
