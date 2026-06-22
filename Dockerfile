# --- Stage 1: Build the Rust Backend ---
FROM rust:alpine AS rust-builder
RUN apk add --no-cache musl-dev

WORKDIR /app

# Cache dependencies by building a dummy project
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/rustpad*

# Copy actual source code and build it
COPY src ./src
RUN cargo build --release

# --- Stage 2: Fetch Frontend Node Modules ---
FROM node:22-alpine AS node-builder
WORKDIR /app
COPY package*.json ./
RUN npm ci --omit=dev

# --- Stage 3: Final Runtime Container ---
FROM alpine:latest

WORKDIR /app

# Create a non-root user matching UID 1000
RUN addgroup -g 1000 rustpad && \
    adduser -u 1000 -G rustpad -s /bin/sh -D rustpad

# Copy compiled Rust binary
COPY --from=rust-builder /app/target/release/rustpad /app/rustpad

# Copy node modules (frontend dependencies)
COPY --from=node-builder /app/node_modules /app/node_modules

# Copy static frontend public assets
COPY public /app/public

# Setup data and asset directories with correct ownership
RUN mkdir -p /app/data /app/public/Assets && \
    chown -R rustpad:rustpad /app

USER rustpad

# Mount data volume
VOLUME /app/data

EXPOSE 3000

CMD ["/app/rustpad"]
