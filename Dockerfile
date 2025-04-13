# Use official Windows Server Core image with build tools
FROM mcr.microsoft.com/windows/servercore:ltsc2022

# Install Chocolatey
RUN powershell -NoProfile -InputFormat None -ExecutionPolicy Bypass -Command \
    Set-ExecutionPolicy Bypass -Scope Process; \
    [System.Net.ServicePointManager]::SecurityProtocol = 'Tls12'; \
    iex ((New-Object System.Net.WebClient).DownloadString('https://chocolatey.org/install.ps1'))

# Install Rust, Chrome, and ChromeDriver
RUN choco install -y rust \
    googlechrome \
    chromedriver

# Set PATH (ChromeDriver goes to C:\ProgramData\chocolatey\bin)
ENV PATH="C:\\ProgramData\\chocolatey\\bin;C:\\Users\\ContainerUser\\.cargo\\bin;$PATH"

# Create app folder
WORKDIR /app

# Copy project and build
COPY . .
RUN cargo build --release

# Expose port for ChromeDriver
EXPOSE 9515
EXPOSE 8080

CMD powershell -Command \
    "Start-Process chromedriver -ArgumentList '--port=9515' -NoNewWindow; \
    Start-Sleep -Seconds 3; \
    .\\target\\release\\instant_dmv_backend.exe"
