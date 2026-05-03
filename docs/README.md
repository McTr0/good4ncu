# Good4NCU 校园AI信息发布平台

> 智能校园信息发布与交易平台，支持AI智能助手、语义搜索、实时聊天。

**免责声明：** 本产品仅做信息发布，无担保和资金中介，也不收手续费。所有交易风险自担。

## Documentation Order

按下面顺序阅读文档，避免在多个旧计划之间来回切换：

1. [PLAN.md](./PLAN.md) — 当前唯一的执行计划与优先级来源
2. [ARCHITECTURE_AUDIT.md](./ARCHITECTURE_AUDIT.md) — 架构诊断、风险和演进方向
3. [SPEC_PLANS.md](./SPEC_PLANS.md) — 子系统专项计划，如 UUID 迁移与聊天验证
4. [DEVELOPER.md](./DEVELOPER.md) / [CONTRIBUTING.md](./CONTRIBUTING.md) — 开发流程、代码规范、提交流程

## 功能特性

| 模块 | 功能 |
|------|------|
| **用户认证** | 注册（邮箱@email.ncu.edu.cn）、登录、JWT刷新 |
| **商品发布** | AI识别商品属性、智能定价建议、分类管理 |
| **商品搜索** | 全文搜索 + 向量语义相似度检索（RAG） |
| **即时通讯** | 买卖双方聊天、连接握手（请求/接受/拒绝）、WebSocket实时推送 |
| **AI助手** | 商品咨询、还价建议、交易引导 |
| **订单管理** | 创建订单、支付超时热插拔重架、7天自动收货、状态机流转 |
| **收藏夹** | 关注商品、下架实时提醒、个性化收藏夹 |
| **管理后台** | 数据大盘、全量操作审计(Audit Logs)、用户封禁、强制下架、权限管理 |

## 环境要求

| 依赖 | 版本要求 | 说明 |
|------|----------|------|
| Rust | 2021 edition | `rustup update` 升级 |
| Flutter | SDK ≥ 3.x | 移动端开发 |
| PostgreSQL | ≥ 14 + pgvector | `CREATE EXTENSION vector` |
| Gemini / MiniMax API | 二选一 | LLM + Embedding |

## 快速启动

### 1. 配置环境变量

```bash
cp .env.example .env
# 编辑 .env 填入实际值
```

**必需变量：**

| 变量 | 说明 |
|------|------|
| `DATABASE_URL` | PostgreSQL 连接串，如 `postgres://user:pass@localhost:5432/good4ncu` |
| `GEMINI_API_KEY` | Google Gemini API Key（用于LLM和Embedding） |
| `JWT_SECRET` | JWT签名密钥，最少32字符 |

**可选变量（可配置到 `good4ncu.toml`）：**

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `LLM_PROVIDER` | `gemini` | `gemini` 或 `minimax` |
| `VECTOR_DIM` | `768` | Embedding 向量维度 |
| `SERVER_PORT` | `3000` | HTTP 服务端口 |

### 2. 创建数据库

```bash
# 连接到 PostgreSQL
psql $DATABASE_URL

# 创建数据库（若不存在）
CREATE DATABASE good4ncu;
\c good4ncu

# 启用向量检索扩展
CREATE EXTENSION vector;

# 运行迁移
# （应用启动时 sqlx 会在首次连接时自动创建表）
```

### 3. 启动后端

```bash
# 开发模式（热重载需手动重启）
cargo run

# 发行版
cargo build --release
./target/release/good4ncu
```

启动成功：
```
Web Server started at http://127.0.0.1:3000
```

### 4. 启动移动端

```bash
cd mobile

# 安装依赖
flutter pub get

# 运行（连接本地后端）
flutter run

# 或指定后端地址
flutter run --dart-define=API_BASE_URL=http://YOUR_IP:3000
```

## 配置文件（TOML）

除环境变量外，非敏感配置可写入 `good4ncu.toml`（环境变量优先级高于配置项）：

```bash
cp config.toml.example good4ncu.toml
# 编辑 good4ncu.toml
```

**配置搜索路径（按优先级）：**
1. `CONFIG_FILE` 环境变量指定的路径
2. `./good4ncu.toml`（项目根目录）
3. `./config/good4ncu.toml`（配置目录）

**主要配置项：**

```toml
[server]
host = "0.0.0.0"
port = 3000

[llm]
provider = "gemini"
vector_dim = 768

[rate_limit]
max_requests = 100
window_secs = 60

[marketplace]
conversation_history_limit = 10
price_tolerance = 0.50
categories = ["electronics", "books", "digitalAccessories", "dailyGoods", "clothingShoes", "other"]

[auth]
access_token_ttl_secs = 86400      # 24小时
refresh_token_ttl_secs = 604800     # 7天
```

详见 `config.toml.example`。

## CLI 管理命令

```bash
# 提升用户为管理员
cargo run admin promote <username>
```

## 项目结构

```
good4ncu/
├── src/                    # Rust 后端
│   ├── main.rs            # 入口：DB初始化、LLM配置、ServiceManager、HTTP服务
│   ├── config.rs          # 配置加载（env + TOML）
│   ├── config/file.rs     # TOML 文件配置提供方
│   ├── db.rs              # PostgreSQL + pgvector 初始化
│   ├── cli.rs             # 交互式 CLI（admin promote 等）
│   ├── utils.rs           # 金额转换：yuan_to_cents()、cents_to_yuan()
│   ├── api/               # REST API（Axum）
│   │   ├── auth.rs        # JWT 注册/登录/刷新
│   │   ├── listings.rs    # 商品 CRUD + AI识别 + 搜索
│   │   ├── orders.rs      # 订单管理 + 状态机流转
│   │   ├── user.rs        # 用户资料、我的商品、用户搜索
│   │   ├── user_chat.rs   # 买卖双方聊天（连接握手、消息收发）
│   │   ├── ws.rs          # WebSocket 处理器 + 广播
│   │   ├── conversations.rs # 会话列表 + 分页
│   │   ├── negotiate.rs   # 还价协商（HITL 工作流）
│   │   ├── notifications.rs # 通知列表、标记已读
│   │   ├── watchlist.rs   # 心愿单增删
│   │   ├── recommendations.rs # 推荐 feed + 相似商品（pgvector）
│   │   ├── upload.rs      # OSS 上传凭证生成
│   │   ├── admin.rs       # 管理后台（封禁、下架、审计日志）
│   │   ├── metrics.rs     # Prometheus /metrics 端点
│   │   └── stats.rs       # 公开站点统计
│   ├── agents/            # AI Agent（Rig 框架）
│   │   ├── router.rs      # 意图分类（轻量级决策树）
│   │   ├── tools.rs       # Agent 工具集
│   │   ├── models.rs      # 领域模型
│   │   └── negotiate.rs   # 议价 Agent（HitlRequest、HumanApprovalTool）
│   ├── llm/              # LLM 提供商
│   │   ├── mod.rs         # LlmProvider trait、PREAMBLE 常量
│   │   ├── gemini.rs      # Google Gemini（chat + embeddings）
│   │   └── minimax.rs     # MiniMax（chat + Gemini embeddings）
│   ├── repositories/      # 数据访问层（trait + Postgres 实现）
│   │   ├── traits.rs     # Repository trait 定义
│   │   ├── auth_repo.rs
│   │   ├── chat_repo.rs
│   │   ├── listing_repo.rs
│   │   ├── order_repo.rs
│   │   └── user_repo.rs
│   ├── services/          # 业务逻辑 + 事件循环 + Worker
│   │   ├── mod.rs        # ServiceManager、BusinessEvent、事件循环
│   │   ├── order.rs       # 订单服务（创建、状态转换、原子性）
│   │   ├── chat.rs        # 聊天服务（连接生命周期、消息历史）
│   │   ├── notification.rs # 通知服务
│   │   ├── admin.rs       # AdminService（审计日志、封禁、下架）
│   │   ├── product.rs     # ProductService（已禁用）
│   │   ├── settlement.rs  # SettlementService（已禁用）
│   │   ├── hitl_expire.rs # 议价超时 Worker（48h，10min 扫描）
│   │   └── order_worker.rs # 订单生命周期 Worker（30m 支付超时、7d 自动确认）
│   └── middleware/        # Axum 中间件
│       └── rate_limit/    # 令牌桶限流（本地内存 / Redis）
├── migrations/            # SQL 迁移脚本（序号执行）
├── mobile/               # Flutter 移动端
│   └── lib/
│       ├── main.dart     # 应用入口
│       ├── pages/        # 页面（home、login、chat、listing_detail 等 17 个）
│       ├── services/     # API 客户端（按领域拆分，13 个服务）
│       ├── providers/    # Provider 状态管理（ChatNotifier 等）
│       ├── l10n/         # 国际化（ARB → Dart）
│       └── theme/        # 主题颜色、尺寸常量
├── good4ncu.toml        # 配置文件（不提交）
├── config.toml.example   # 配置模板（可提交）
└── .env                  # 环境变量（不提交）
```

## 数据库

**PostgreSQL + pgvector**：关系型数据与向量检索共用一个数据库。

| 表 | 说明 |
|----|------|
| `users` | 用户（id, username, email, password_hash, role, status） |
| `inventory` | 商品（标题、分类、品牌、成色、建议价、缺陷描述、状态、owner_id） |
| `chat_connections` | 聊天连接（买卖双方握手状态：pending/connected/rejected） |
| `chat_messages` | 聊天消息（sender, receiver, content, is_agent, 已读时间戳） |
| `orders` | 订单（状态机：pending → paid → shipped → completed/cancelled） |
| `documents` | 向量文档（listing_id + pgvector embedding，供语义搜索） |
| `watchlist` | 心愿单（user_id, listing_id） |
| `notifications` | 通知（user_id, event_type, title, body, related_order_id） |
| `hitl_requests` | 议价请求（propose_price, status: pending/approved/rejected/expired） |
| `refresh_tokens` | JWT 刷新令牌（user_id, token_hash, expires_at） |
| `admin_audit_logs` | 管理员操作审计（admin_id, action, target, old/new_value） |

**金额存储**：所有价格存为 `i32` 整数（分），显示时除以 100 转为元。

## 开发指南

详见 [DEVELOPER.md](DEVELOPER.md)，包括：
- 分支管理、代码审查流程
- Rust / Flutter 代码规范
- 测试策略（单元测试 + 集成测试）
- 多 Agent 协作（rust-reviewer、flutter-reviewer、security-reviewer 等）
