FROM rust:1.85-slim

# Create a new empty shell project directory
WORKDIR /app

# Install required build tools and OpenSSL development headers
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies first (for faster rebuilds)
COPY Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -r src

# Copy actual source code
COPY . .

# Build actual project
RUN cargo build --release

# Copy the binary
COPY ./target/release/instant_dmv_backend .

# Make sure it's executable w/ chmod
RUN chmod +x instant_dmv_backend

# Use ENTRYPOINT
ENTRYPOINT ["./instant_dmv_backend"]
