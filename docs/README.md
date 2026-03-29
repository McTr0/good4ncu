# Good4NCU 校园AI信息发布平台

> 智能校园信息发布与交易平台，支持AI智能助手、语义搜索、实时聊天。

**免责声明：** 本产品仅做信息发布，无担保和资金中介，也不收手续费。所有交易风险自担。

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
host = "127.0.0.1"
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
│   ├── main.rs            # 入口：DB初始化、LLM配置、事件总线、HTTP服务
│   ├── config.rs          # 配置加载（env + TOML）
│   ├── config/file.rs     # TOML 配置结构定义
│   ├── db.rs              # PostgreSQL + pgvector 初始化
│   ├── api/               # REST API（Axum）
│   │   ├── auth.rs        # JWT 注册/登录/刷新
│   │   ├── listings.rs    # 商品 CRUD + AI识别
│   │   ├── orders.rs      # 订单管理
│   │   ├── user.rs        # 用户资料
│   │   ├── user_chat.rs   # 买卖双方聊天
│   │   ├── ws.rs          # WebSocket 处理器
│   │   ├── negotiate.rs   # 还价协商
│   │   ├── upload.rs      # OSS 上传凭证
│   │   └── admin.rs       # 管理后台
│   ├── agents/            # AI Agent（Rig框架）
│   │   ├── router.rs      # 意图分类（轻量级决策树）
│   │   └── tools.rs       # Agent工具集
│   ├── llm/               # LLM 提供商
│   │   ├── gemini.rs       # Google Gemini
│   │   └── minimax.rs     # MiniMax（Chat + Gemini Embedding）
│   ├── repositories/      # 数据访问层（trait + Postgres实现）
│   │   ├── traits.rs      # Repository trait 定义
│   │   ├── listing_repo.rs
│   │   ├── user_repo.rs
│   │   ├── chat_repo.rs
│   │   └── auth_repo.rs
│   └── services/          # 业务逻辑 + 事件循环
│       ├── mod.rs          # ServiceManager、BusinessEvent、事件循环
│       ├── order.rs        # 订单服务
│       ├── chat.rs         # 聊天服务
│       └── notification.rs  # 通知服务
├── migrations/             # SQL 迁移脚本（按序号执行）
├── mobile/                # Flutter 移动端
│   └── lib/
│       ├── main.dart      # 应用入口
│       ├── pages/         # 页面（home, chat, listing_detail, profile 等）
│       ├── services/      # API 客户端（按领域拆分）
│       ├── providers/     # Provider 状态管理
│       ├── l10n/           # 国际化（ARB → Dart）
│       └── theme/          # 主题颜色、尺寸常量
├── good4ncu.toml         # 配置文件（不提交到 git）
├── config.toml.example    # 配置模板（可提交）
└── .env                   # 环境变量（不提交到 git）
```

## 数据库

**PostgreSQL + pgvector**：关系型数据与向量检索共用一个数据库。

| 表 | 说明 |
|----|------|
| `users` | 用户（id, username, email, password_hash, role） |
| `inventory` | 商品（标题、分类、品牌、成色、建议价、状态） |
| `chat_connections` | 聊天连接（买卖双方握手状态） |
| `chat_messages` | 聊天消息 |
| `orders` | 订单（状态机：pending → paid → shipped → confirmed） |
| `documents` | 向量文档（listing_id + pgvector embedding） |
| `watchlist` | 收藏夹 |

**金额存储**：所有价格存为 `i32` 整数（分），显示时除以 100 转为元。

## 开发指南

详见 [DEVELOPER.md](DEVELOPER.md)，包括：
- 分支管理、代码审查流程
- Rust / Flutter 代码规范
- 测试策略（单元测试 + 集成测试）
- 多 Agent 协作（rust-reviewer、flutter-reviewer、security-reviewer 等）
