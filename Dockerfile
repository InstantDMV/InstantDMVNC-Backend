FROM debian:bookworm

# Install Rust
RUN apt-get update && apt-get install -y curl build-essential \
    && curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain 1.85.0 \
    && . "$HOME/.cargo/env"

ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Install dependencies for Chrome & headless usage
RUN apt-get update && apt-get install -y \
    unzip \
    wget \
    jq \
    gnupg \
    ca-certificates \
    pkg-config \
    libssl-dev \
    netcat-openbsd \
    fonts-liberation \
    libx11-6 \
    libx11-xcb1 \
    libxcb1 \
    libxcomposite1 \
    libxcursor1 \
    libxdamage1 \
    libxext6 \
    libxfixes3 \
    libxi6 \
    libxtst6 \
    libnss3 \
    libxrandr2 \
    libasound2 \
    libatk-bridge2.0-0 \
    libgtk-3-0 \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

# Install Chrome for Testing and ChromeDriver (matching versions)
RUN CHROME_VERSION=$(curl -sSL https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json \
    | jq -r '.channels.Stable.version') && \
    wget -O /tmp/chrome-linux64.zip "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/${CHROME_VERSION}/linux64/chrome-linux64.zip" && \
    unzip /tmp/chrome-linux64.zip -d /opt && \
    ln -s /opt/chrome-linux64/chrome /usr/bin/google-chrome && \
    wget -O /tmp/chromedriver.zip "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/${CHROME_VERSION}/linux64/chromedriver-linux64.zip" && \
    unzip /tmp/chromedriver.zip -d /usr/local/bin && \
    mv /usr/local/bin/chromedriver-linux64/chromedriver /usr/local/bin/chromedriver && \
    chmod +x /usr/local/bin/chromedriver && \
    rm -rf /tmp/*.zip /usr/local/bin/chromedriver-linux64

# Copy and build Rust app
COPY . .
RUN . "$HOME/.cargo/env" && cargo build --release

EXPOSE 60103
EXPOSE 8080

CMD bash -c '\
    chromedriver --port=60103 & \
    while ! nc -z localhost 60103; do echo "Waiting for ChromeDriver..."; sleep 0.5; done; \
    echo "ChromeDriver is ready."; \
    ./target/release/instant_dmv_backend'
