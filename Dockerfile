# 多阶段构建 - 构建阶段
FROM rust:1.89-slim AS builder

# 添加 musl 目标
RUN rustup target add x86_64-unknown-linux-musl

# 设置工作目录
WORKDIR /app

# 复制源代码
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# 设置编译选项（如需 OpenSSL 可自行添加）
ENV PKG_CONFIG_ALLOW_CROSS=1
ENV RUSTFLAGS="-C link-arg=-s -C opt-level=z -C target-feature=+crt-static"

# 静态链接编译 - 使用 musl 目标
RUN touch src/main.rs && \
    cargo build --release --target x86_64-unknown-linux-musl

# 运行阶段 - 使用scratch
FROM scratch

LABEL maintainer="AptS-1547 <apts-1547@esaps.net>"
LABEL description="Porture is a minimal, programmable port forwarder written in Rust."
LABEL version="0.1.0"
LABEL homepage="https://github.com/AptS-1547/porture"
LABEL license="MIT"

# 从构建阶段复制二进制文件 (使用 musl 目标路径)
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/porture /porture

# 暴露端口（如有多个端口可自行添加）
EXPOSE 8080

# 设置环境变量（可根据实际需要调整）
ENV DOCKER_ENV=1
ENV RUST_LOG=info

# 启动命令
ENTRYPOINT ["/porture"]