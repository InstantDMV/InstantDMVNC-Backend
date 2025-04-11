FROM rust:1.85-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    curl \
    unzip \
    gnupg \
    wget \
    ca-certificates \
    netcat \
    && rm -rf /var/lib/apt/lists/*

# Install Chrome
RUN wget -q -O - https://dl.google.com/linux/linux_signing_key.pub | apt-key add - \
    && echo "deb [arch=amd64] http://dl.google.com/linux/chrome/deb/ stable main" > /etc/apt/sources.list.d/google-chrome.list \
    && apt-get update && apt-get install -y google-chrome-stable

# Install matching ChromeDriver
RUN CHROME_VERSION=$(google-chrome --version | grep -oP '\d+\.\d+\.\d+') && \
    CHROMEDRIVER_VERSION=$(curl -s "https://chromedriver.storage.googleapis.com/LATEST_RELEASE_${CHROME_VERSION}") && \
    wget -O /tmp/chromedriver.zip "https://chromedriver.storage.googleapis.com/${CHROMEDRIVER_VERSION}/chromedriver_linux64.zip" && \
    unzip /tmp/chromedriver.zip -d /usr/local/bin && \
    chmod +x /usr/local/bin/chromedriver && \
    rm /tmp/chromedriver.zip

# Cache Rust dependencies
COPY Cargo.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -r src

# Copy actual source code
COPY . .
RUN cargo build --release

# Expose the ChromeDriver and backend ports
EXPOSE 60103
EXPOSE 8080

# Entrypoint: Start chromedriver, wait until it's ready, then start Rust backend
CMD bash -c '\
  chromedriver --port=60103 & \
  while ! nc -z localhost 60103; do echo "Waiting for ChromeDriver..."; sleep 0.5; done; \
  echo "ChromeDriver is ready."; \
  ./target/release/instant_dmv_backend'
