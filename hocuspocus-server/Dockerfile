FROM node:20-alpine

WORKDIR /app

COPY package.json ./
RUN npm install

COPY . .

ENV PORT=1234
EXPOSE 1234

CMD ["node", "index.js"]
