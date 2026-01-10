# Build stage
FROM node:20-alpine AS builder

WORKDIR /app

# Enable corepack for yarn
RUN corepack enable

# Copy workspace package files
COPY package.json yarn.lock .yarnrc.yml ./
COPY .yarn ./.yarn

# Copy relevant packages
COPY applications/web/package.json ./applications/web/
COPY applications/shared/package.json ./applications/shared/

# Install dependencies
RUN yarn install --immutable

# Copy source code
COPY applications/web ./applications/web
COPY applications/shared ./applications/shared

# Build the web app
WORKDIR /app/applications/web
RUN yarn build

# Runtime stage
FROM nginx:alpine

# Copy built files
COPY --from=builder /app/applications/web/dist /usr/share/nginx/html

# Copy nginx configuration
COPY applications/web/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
