# -------------------------------------------------------
# 🚧 Stage 1: Build the Rust binary using Rust 1.74
# -------------------------------------------------------
FROM rustlang/rust:nightly AS builder

# Set working directory
WORKDIR /app

# Copy source code
COPY . .

# Build in release mode
RUN cargo build --release

# -------------------------------------------------------
# 🏃 Stage 2: Create lightweight container to run binary
# -------------------------------------------------------
FROM debian:bookworm-slim

# Copy binary
COPY --from=builder /app/target/release/db-server /usr/local/bin/db-server



# Run the TCP server on port 4000
EXPOSE 4000



ENTRYPOINT ["db-server"]
CMD ["4000"]
