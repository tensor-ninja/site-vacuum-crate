FROM rust:slim-bullseye as builder

WORKDIR /usr/src/site-vacuum

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy source code
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim

# Install Chromium and chromedriver dependencies
RUN apt-get update && apt-get install -y \
    wget \
    curl \
    gnupg \
    ca-certificates \
    unzip \
    chromium \
    chromium-driver \
    --no-install-recommends \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /usr/src/site-vacuum/target/release/site-vacuum /usr/local/bin/site-vacuum

# Create a script to start chromedriver and the application
RUN echo '#!/bin/bash\n\
/usr/local/bin/site-vacuum\n' > /usr/local/bin/start.sh \
    && chmod +x /usr/local/bin/start.sh

# Expose the application port
EXPOSE 8000

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/start.sh"] 