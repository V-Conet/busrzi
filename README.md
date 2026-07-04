# busrzi

一个简化版的不蒜子（busuanzi）站点计数服务，基于 Rust + Axum + Redis/Valkey。

## 功能

- `POST /api/counter`：记录页面访问，返回站点 PV、页面 PV、站点 UV
- `GET /js`：返回浏览器嵌入脚本，自动更新页面计数
- Redis/Valkey 持久化，支持可配置 TTL
- 跨域支持、缓存控制、结构化日志

## 快速开始

### 1. 准备 Valkey

```bash
docker run -d --name busrzi-valkey -p 6379:6379 \
  valkey/valkey:9.1-alpine
```

### 2. 配置环境变量

```bash
# 编辑 .env 并修改 REDIS_URL
cp .env.example .env
```

### 3. 运行

```bash
cargo run --release
```

服务默认监听 `0.0.0.0:8080`。

## 前端嵌入

在网页中引入脚本并放置计数元素：

```html
<script defer src="http://localhost:8080/js"></script>

<p>本文总阅读量 <span id="busrzi_page_pv">Loading</span> 次</p>
<p>本站总访问量 <span id="busrzi_site_pv">Loading</span> 次</p>
<p>本站总访客数 <span id="busrzi_site_uv">Loading</span> 人</p>
```

## API

### POST /api/counter

请求：

```json
{
  "url": "https://example.com/blog/post",
  "is_new_uv": true
}
```

响应：

```json
{
  "ok": true,
  "result": {
    "site_uv": 1,
    "site_pv": 10,
    "page_pv": 3
  },
  "info": "counters updated"
}
```

### GET /js

返回 `application/javascript`，用于网页嵌入。

## 环境变量

| 变量 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `REDIS_URL` | 是 | - | Redis/Valkey 连接 URL |
| `PORT` | 否 | `8080` | busrzi 监听端口，`PORT` |
| `SCRIPT_PATH` | 否 | `assets/client.js` | 客户端 JS 文件路径 |
| `TTL_DAYS` | 否 | `90` | 计数数据 TTL（天），`0` 表示永不过期 |
| `RUST_LOG` | 否 | `info` | 日志级别，例如 `busrzi=debug` |

## 项目结构

```
busrzi/
├── src/
│   ├── main.rs           # 服务入口
│   ├── config.rs         # 环境变量配置
│   ├── counter.rs        # 计数逻辑与 URL 规范化
│   ├── error.rs          # 错误处理
│   └── handlers/         # HTTP 处理器
│       ├── mod.rs
│       ├── log.rs
│       └── script.rs
├── assets/client.js      # 浏览器嵌入脚本
├── scripts/              # 压测脚本
│   └── bench_plot.py
├── docker-compose.yml
└── .env.example
```

## 测试

```bash
# 单元测试
cargo test

# 手动验证
curl http://localhost:8080/js
curl -X POST http://localhost:8080/api/counter \
  -H "Content-Type: application/json" \
  -d '{"url":"https://example.com/blog/post","is_new_uv":true}'
```

## 压力测试

```bash
cargo install oha

# 需要 matplotlib pandas
./scripts/bench_plot.py
```

压测结果默认输出到 `bench_results/` 目录。

## Docker 部署

```bash
docker compose up -d
```

## 灵感来源

- [不蒜子](https://busuanzi.ibruce.info/)

## 许可证

MIT