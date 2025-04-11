FROM rust:1.85-slim

# Create a new empty shell project directory
WORKDIR /app

# Cache dependencies first (for faster rebuilds)
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -r src

# Copy actual source code
COPY . .

# Build actual project
RUN cargo build --release

CMD ["./target/debug/instant_dmv_backend"]
