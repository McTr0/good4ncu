# Good4NCU 项目审计报告

> **审计日期**: 2026-03-29  
> **范围**: 后端 (Rust/Axum) + 移动端 (Flutter) + 数据库 (PostgreSQL/pgvector) + 基础设施  
> **版本**: v0.1.0

---

## 一、项目概况

Good4NCU 是面向中国大学校园的二手交易平台，采用 **AI Agent 驱动** 的架构。

| 层级 | 技术栈 | 规模 |
|------|--------|------|
| **后端** | Rust 2021 + Axum 0.8 + sqlx 0.8 | ~17 API 模块, ~11 服务模块 |
| **移动端** | Flutter (Dart SDK ^3.10.8) | ~17 页面, ~26 服务类 |
| **数据库** | PostgreSQL + pgvector | 11 迁移文件, 10+ 表 |
| **AI/LLM** | rig-core 0.33 + Gemini/MiniMax | Intent Router + Marketplace Agent |
| **部署** | Docker (distroless) | 多阶段构建 |

---

## 二、架构评估

### 2.1 后端架构

**优点**:
- ✅ 分层清晰: API → Service → Repository 三层架构
- ✅ 事件驱动: 通过 `mpsc` channel 解耦业务事件
- ✅ 安全头部: 自动注入 HSTS、X-Frame-Options 等
- ✅ 可观测性: Prometheus 指标 + 结构化 JSON 日志
- ✅ 配置管理: TOML + 环境变量双层配置，秘钥在 Debug 输出中被脱敏

**问题与隐患**:

| ID | 严重性 | 问题 | 影响 |
|----|--------|------|------|
| B-01 | 🔴 严重 | `user_chat.rs` 达 57KB，职责过重 | 维护困难，耦合度极高 |
| B-02 | 🔴 严重 | `AppState` 为 God Object（含 secrets/infra/agents/5个repo） | 跨模块依赖不清，测试困难 |
| B-03 | 🟠 高 | Repository 不支持事务传递 (`order.rs:110-111` 注释承认) | 跨表操作的原子性依赖裸 SQL |
| B-04 | 🟠 高 | `format!` 构建 SQL (`order.rs:295`, `order.rs:202`) | 虽为内部字段名非用户输入，但违背最佳实践 |
| B-05 | 🟡 中 | `SettlementService` 所有方法返回 `Disabled` | 死代码，增加认知负担 |
| B-06 | 🟡 中 | `normalize_path` 每次请求编译 3 个 Regex | 性能浪费，应用 `lazy_static` |
| B-07 | 🟡 中 | Event Bus 阻塞发送 (`handle_chat` 用 `.send().await`) 但 SSE 用 `.try_send()` | 行为不一致，SSE 路径可能丢失事件 |

### 2.2 移动端架构

**优点**:
- ✅ Shell/Detail 路由分离，底栏行为正确
- ✅ AI 助手全局化，通过 Provider 管理状态
- ✅ 自动 Token 刷新 + 401 拦截跳登录
- ✅ 国际化 (i18n) 完整覆盖

**问题与隐患**:

| ID | 严重性 | 问题 | 影响 |
|----|--------|------|------|
| F-01 | 🔴 严重 | `admin_page.dart` 达 39KB，单文件包含整个管理后台 | 编译缓慢，不可维护 |
| F-02 | 🔴 严重 | `user_chat_page.dart` 达 26KB，混合 UI + 业务逻辑 + WS 处理 | 难以测试和复用 |
| F-03 | 🟠 高 | 无自动 Token 刷新中间件 — 401 直接跳登录 | 用户体验差，长时间使用后突然被登出 |
| F-04 | 🟠 高 | `OverlayEntry` 的 Provider 注入是手动传递 | 容易遗漏，已导致过 ProviderNotFound 崩溃 |
| F-05 | 🟠 高 | GoRouter 的 `redirect` 每次导航都发网络请求 (`_isAdmin()`) | 首次加载慢，网络差时卡死 |
| F-06 | 🟡 中 | 多个页面直接 `new XxxService()` 创建实例而非注入 | 无法 mock 测试，违背 DI 原则 |
| F-07 | 🟡 中 | `chat_page.dart` 30KB — AI 聊天页面同样过大 | 同 F-01 |
| F-08 | 🟡 中 | 无 Flutter 单元测试和 Widget 测试 | 质量无保障 |

### 2.3 数据库架构

**优点**:
- ✅ 外键约束 + CHECK 约束保证数据完整性
- ✅ `pgvector` HNSW 索引支持语义搜索
- ✅ 迁移文件版本化管理

**问题与隐患**:

| ID | 严重性 | 问题 | 影响 |
|----|--------|------|------|
| D-01 | 🔴 严重 | 主键为 `TEXT` 类型 (users.id, inventory.id, orders.id) | 索引效率低，JOIN 性能差，无法利用 UUID 原生类型 |
| D-02 | 🟠 高 | `chat_messages.sender` FK 指向 `users(id)`，但 Agent 消息 sender = `"assistant"` | FK 约束被绕过或 Agent 必须假装是 user 行 |
| D-03 | 🟠 高 | `documents.embedding` 固定 `vector(768)` 硬编码 | 切换嵌入模型（如 MiniMax 的 1536 维度）需要手动迁移 |
| D-04 | 🟠 高 | `chat_connections` 唯一约束 `(requester_id, receiver_id)` 单向 | A→B 和 B→A 会创建两个连接 |
| D-05 | 🟡 中 | `image_data` 和 `audio_data` 存储在 `chat_messages` 表中 (Base64 TEXT) | 表膨胀严重，查询性能下降 |
| D-06 | 🟡 中 | 无 `updated_at` 列 | 无法追踪记录修改时间 |
| D-07 | 🟡 中 | `inventory.defects` 为 `TEXT` 存储逗号分隔值 | 无法高效查询和索引 |

---

## 三、安全审计

### 3.1 认证与授权

| 检查项 | 状态 | 说明 |
|--------|------|------|
| 密码哈希 | ✅ | Argon2（阻塞任务中执行） |
| JWT 签发 | ✅ | 24h 过期 + Refresh Token 轮换 |
| JWT 密钥长度 | ✅ | 强制 ≥ 32 字符 |
| 新旧密钥兼容 | ✅ | `jwt_secret_old` 回退机制 |
| 密码长度限制 | ✅ | 8-128 字符 |
| 用户名枚举防护 | ✅ | 错误信息不区分"用户不存在"和"密码错误" |
| Admin 路由保护 | ⚠️ | 仅前端 redirect 检查角色，API 层部分 handler 可能缺少 role 校验 |
| CSRF 防护 | ❌ | 无 — 依赖 Bearer Token 但 WebSocket 端存在风险 |
| Rate Limit 绕过 | ⚠️ | 使用 IP 限流，但代理后 IP 可能全部为代理地址 |

### 3.2 数据安全

| 检查项 | 状态 | 说明 |
|--------|------|------|
| SQL 注入 | ✅ | 全部使用 sqlx 参数化查询（`format!` 仅用于列名枚举） |
| XSS | ✅ | API 返回 JSON，X-Content-Type-Options: nosniff |
| 敏感数据日志泄露 | ✅ | `ApiError::Internal` 隐藏了具体错误，只暴露 "服务器内部错误" |
| .env 文件保护 | ⚠️ | `.env` 存在于项目根目录 (580 字节)，需确认 `.gitignore` 覆盖 |
| Base64 媒体存储 | ❌ | 聊天中的图片/音频以 Base64 存入 DB，一旦泄露即获得原始数据 |
| 内容审核 | ✅ | 文本审核 + 图片审核 worker |

### 3.3 关键安全建议

**S-01 [严重]**: `.env` 文件必须在 `.gitignore` 中，且不应出现在版本库历史中。立即检查 `git log --all --full-history -- .env`。

**S-02 [高]**: Admin API handler（`ban_user`, `update_order_status` 等）应在 handler 层显式验证 `role == "admin"`，不能仅依赖前端路由保护。

**S-03 [高]**: `Dockerfile` 中使用 `distroless` 镜像，但同时有 `RUN useradd` 和 `wget` 健康检查命令 — `distroless` 镜像不包含这些工具。**构建将失败**。

---

## 四、性能分析

### 4.1 热点路径

| 路径 | 问题 | 建议 |
|------|------|------|
| `GoRouter.redirect` | 每次导航均 `await getLoginStatus()` + `await _isAdmin()` (后者发网络请求) | 缓存角色信息到本地，仅登录/刷新时更新 |
| `normalize_path()` | 每请求编译 3 个正则表达式对象 | 使用 `lazy_static!` 或 `once_cell` 预编译 |
| `chat_messages` 查询 | Base64 大字段导致全表扫描慢 | 将媒体数据迁移到 OSS，DB 只存 URL |
| `documents` 向量搜索 | HNSW 索引正确，但表无分区 | 商品量超 10 万后考虑分区 |
| HTTP Client | `BaseService` 使用 `static final http.Client` 单例 | ✅ 正确，连接复用 |

### 4.2 并发与资源

| 项目 | 当前值 | 评估 |
|------|--------|------|
| DB 连接池 | min=2, max=20 | ✅ 合理 |
| Event Bus 容量 | 2048 | ✅ 合理 |
| Rate Limit | 100 req / 60s | ⚠️ 偏松，建议按端点细分 |
| 请求体限制 | 10 MB | ⚠️ 因 Base64 媒体上传需要，但偏大 |
| HTTP 超时 (移动端) | GET 15s, POST 30s | ✅ 合理 |

---

## 五、Dockerfile 问题

当前 Dockerfile **无法成功构建**。

```diff
- FROM gcr.io/distroless/cc-debian12 AS runtime
+ FROM debian:bookworm-slim AS runtime

- RUN useradd --create-home --shell /bin/bash appuser && \
-     chown -R appuser:appuser /home/appuser
+ RUN groupadd -r appuser && useradd -r -g appuser -d /home/appuser -s /sbin/nologin appuser && \
+     mkdir -p /home/appuser && chown -R appuser:appuser /home/appuser

# distroless 没有 shell 也没有 wget
- HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
-     CMD wget --no-verbose --tries=1 --spider http://localhost:3000/api/health || exit 1
+ HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
+     CMD ["/usr/local/bin/good4ncu", "--health-check"]
```

**原因**: `distroless` 镜像不包含 shell、`useradd`、`wget` 等工具。要么改用 `debian:slim`，要么移除所有 shell 依赖命令。

---

## 六、技术债清单

### 6.1 优先级 P0（阻塞发布）

| # | 项目 | 描述 | 投入估计 |
|---|------|------|----------|
| 1 | Dockerfile 修复 | distroless 镜像与 shell 命令冲突 | 0.5h |
| 2 | Admin API 权限校验 | 所有 admin handler 显式验证 role | 2h |
| 3 | 移动端自动 Token 刷新 | BaseService 拦截 401，自动 refresh 后重试 | 4h |

### 6.2 优先级 P1（核心体验）

| # | 项目 | 描述 | 投入估计 |
|---|------|------|----------|
| 4 | 拆分巨型文件 | `user_chat.rs` (57KB), `admin_page.dart` (39KB), `chat_page.dart` (30KB) | 8h |
| 5 | 媒体存储上云 | 聊天图片/音频从 Base64 存 DB 改为 OSS URL | 6h |
| 6 | 数据库主键类型 | `TEXT` → `UUID` 原生类型迁移 | 4h |
| 7 | 双向聊天连接 | 修复 `chat_connections` 唯一约束为双向 | 2h |
| 8 | GoRouter redirect 优化 | 缓存用户角色，避免每次导航发网络请求 | 2h |

### 6.3 优先级 P2（工程质量）

| # | 项目 | 描述 | 投入估计 |
|---|------|------|----------|
| 9 | Flutter 测试覆盖 | Widget 测试 + Service Mock 测试 | 12h |
| 10 | 正则表达式预编译 | `normalize_path` 使用 `lazy_static` | 0.5h |
| 11 | 依赖注入改造 | 页面不再 `new Service()`，改为 Provider 注入 | 6h |
| 12 | Repository 事务支持 | 允许 Repository 方法接受 `&mut Transaction` 参数 | 4h |
| 13 | Event Bus 一致性 | 统一 `send` / `try_send` 策略，增加失败补偿 | 2h |
| 14 | 移除死代码 | `SettlementService` 空壳、`user_center_page.dart` 等 | 1h |
| 15 | `updated_at` 列 | 所有核心表添加 `updated_at` 触发器 | 2h |

---

## 七、演进路线图

### Phase 1: 稳定化（当前 → 2 周内）

**目标**: 消除所有 P0 阻塞项，修复已知崩溃路径。

- [x] Flutter 布局崩溃修复 (StackFit.expand + Positioned guard)
- [x] ProviderNotFoundException 修复 (OverlayEntry provider injection)
- [ ] Dockerfile 修复 (distroless → debian:slim)
- [ ] Admin API 权限校验
- [ ] 自动 Token 刷新
- [ ] 巨型文件拆分
- [ ] GoRouter 优化
- [ ] 双向聊天连接修复

### Phase 2: 数据层升级（2-4 周）

- [ ] 主键类型迁移 `TEXT → UUID`
- [ ] 聊天媒体从 DB 迁移到 OSS（仅存 URL）
- [ ] 添加 `updated_at` 列 + 审计触发器
- [ ] `documents.embedding` 维度参数化
- [ ] `inventory.defects` 改为 `TEXT[]` 数组类型

### Phase 3: 架构现代化（1-2 月）

- [ ] **后端**: Repository 支持事务传递，AppState 拆分为 Sub-State
- [ ] **移动端**: Provider 依赖注入全覆盖，Widget 测试 ≥ 60% 覆盖率
- [ ] **CI/CD**: GitHub Actions 自动化测试 + Docker 构建 + 数据库迁移校验
- [ ] **可观测性**: OpenTelemetry Tracing 替代纯日志
- [ ] **支付集成**: SettlementService 对接真实支付网关

### Phase 4: 规模化准备（3-6 月）

- [ ] Redis 分布式 Rate Limiter（替代进程内 Moka 缓存）
- [ ] 读写分离：向量搜索走只读副本
- [ ] 用户画像系统：基于浏览/购买行为的推荐
- [ ] 消息推送：APNs / FCM 集成
- [ ] 邮箱验证流程完整实现

---

## 八、正面评价

尽管存在技术债，项目整体架构设计是**合理且有远见的**：

1. **AI-first 设计**: Intent Router → LLM Agent 的分层模式有效控制了 LLM Token 消耗
2. **安全意识强**: Argon2 哈希、JWT 密钥轮换、结构化错误消息（不泄露内部信息）
3. **事件驱动**: `mpsc` event bus 为后续微服务拆分留足空间
4. **国际化完备**: 中英双语支持，l10n 覆盖完整
5. **配置层优秀**: TOML + env vars 双层优先级，Debug 输出自动脱敏
6. **内容审核前置**: 文本 + 图片审核在消息持久化前执行

---

> 本审计基于源码静态分析，未进行运行时渗透测试或压力测试。建议在 Phase 1 完成后进行专项安全评估。
