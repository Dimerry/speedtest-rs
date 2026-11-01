# speedtest-rs

自建测速页（替代 LibreSpeed 的 PHP 版），Rust + Axum 后端 + 单文件前端。

- 端口：`5111`
- 二进制：静态 musl 链接，约 1 MB，Docker `scratch` 镜像
- 统计方式：12 次 ping（取中位数区间 + 抖动）、4 线程并发下载 8 秒、3 线程并发上传 8 秒

## API

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/` | 页面 |
| GET | `/api/ping` | 204，用于测延迟 |
| GET | `/api/download?bytes=N` | 流式返回 N 字节（上限 512 MB） |
| POST | `/api/upload` | 丢弃请求体，返回接收字节数 |
| GET | `/api/ip` | 客户端 IP（服务端视角） |
| GET | `/api/config` | 版本与限制信息 |

## 本地构建

```bash
export PATH=$HOME/.cargo/bin:$HOME/toolchain/root/usr/bin:$PATH
export LD_LIBRARY_PATH=$HOME/toolchain/root/usr/lib/x86_64-linux-gnu
cargo build --release --target x86_64-unknown-linux-musl
docker build -t speedtest-rs:1.0.0 .
```

> 本机没有 root，GCC/binutils 是从 Debian 包解包到 `~/toolchain/root` 的便携工具链；
> Rust 工具链在 `~/.cargo`、`~/.rustup`（rustup stable 1.99.1）。

## 运行

```bash
docker run -d --name speedtest --restart unless-stopped -p 5111:5111 speedtest-rs:1.0.0
```

环境变量：`PORT`（默认 5111）。

## 许可证

MIT OR Apache-2.0（双许可，任选其一）
