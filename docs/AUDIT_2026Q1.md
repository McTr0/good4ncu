# Good4NCU 项目架构审计与演进规划

> **更新日期**: 2026-03-29 (Phase 1 初期修复后)
> **受众**: 开发团队与架构师
> **范围**: 后端 (Rust/Axum) + 移动端 (Flutter) + 数据库 (PostgreSQL/pgvector) + AI 层 + WS 层  

---

## 一、项目概况与基线状态

Good4NCU 是面向大学校园的二手交易平台，核心亮点在于**AI Agent 驱动机制**与响应式体验。经过初步治理，目前系统的紧急阻塞漏洞（如 Docker 构建失败、Admin 权限绕开、移动端频繁掉线、单向聊天连接并发错乱）已得到修复。

**核心技术栈**:
- **后端**: Rust 2021 + Axum 0.8 + sqlx 0.8
- **移动端**: Flutter (Dart SDK ^3.10.8) + Provider + GoRouter
- **数据层**: PostgreSQL + pgvector
- **AI/LLM**: rig-core 0.33 + Gemini/MiniMax

---

## 二、架构健康度评估

### 2.1 后端架构 (Axum + sqlx)
**状态**: `良好，但局部过度耦合`
- ✅ **优势**: API → Service → Repository 分层清晰；事件驱动 (`mpsc`) 有效解耦了异步任务；统一的安全头部与 JSON 错误遮蔽。
- ⚠️ **隐患 (巨型类)**: `src/api/user_chat.rs` 高达 1600+ 行（57KB），混杂了模型定义、连接握手、消息处理与 WS 广播，维护认知负担极高。
- ⚠️ **隐患 (God Object)**: `AppState` 汇聚了所有的 Config, Infra, Secrets, Agent 和所有 Repository，违背接口隔离，导致单元测试 Mock 极其困难。
- ⚠️ **隐患 (事务控制)**: Repository 层当前设计无法跨方法透传 `&mut Transaction`，跨表业务（如订单状态流转）仍以裸写 SQL 形式硬编码在 Service 中。

### 2.2 移动端架构 (Flutter)
**状态**: `良好，UI与状态正在逐步解耦`
- ✅ **优势**: Shell/Detail 底栏路由成熟；实现了 401 全局拦截并自动静默刷新 Token；Admin 角色判定已引入本地缓存层。
- ⚠️ **隐患 (巨型页面)**: `admin_page.dart` (近1200行) 和 `user_chat_page.dart` 过于臃肿，UI 树与网络/状态逻辑未分类。
- ⚠️ **隐患 (依赖注入)**: 多数页面仍有硬编码的 `new XxxService()` 实例创建行为，阻碍了基于接口的 Mock 测试体系。
- ⚠️ **隐患 (测试覆盖)**: 缺少自动化 Widget 测试与集成测试。

### 2.3 数据库与存储架构
**状态**: `次优，存有规模化瓶颈`
- ✅ **优势**: `chat_connections` 现已采用双向 `LEAST/GREATEST` 唯一索引避免分裂；外键体系完整。
- 🔴 **危急 (性能与成本)**: `chat_messages` 直接以 Base64 TEXT 字段存储图片与音频，表极易膨胀引起全表扫描性能灾难。
- 🟠 **高 (索引与利用)**: 核心表（如 users, inventory, orders）依然将 UUID 存储为 `TEXT` 类型，而非原生 `UUID`，Join 与内存利用率低。
- 🟡 **中 (AI 演进)**: `documents.embedding` 维度目前硬编码为 `vector(768)`，若切换千问或 MiniMax 高维模型将面临困难。无通用的 `updated_at` 行级追踪。

### 2.4 AI 子系统与并发链路
**状态**: `良好，已补齐关键可观测性`
- ✅ **AI 路由防御**: `IntentRouter` 使用正则表达式+启发式关键词执行 `0 Token` 粗筛与违法词汇阻断，极大地节约了推理成本并避免了 Prompt 注入。
- ✅ **隔离更新**: `UpdateListingTool` 利用 `spawn_blocking` 在安全的非异步闭包中保持 Postgres 事务锁定，完成 DB Update 与重新 Embedding 的原子化。
- ✅ **WS 可观测补强（本轮完成）**: 已为 WS 满载丢弃与僵尸连接清理增加指标计数（`ws_messages_dropped_total`, `ws_stale_connections_pruned_total`），并在通道满载时记录告警日志，便于容量治理。

---

## 三、安全与合规审计

| 检查项 | 状态 | 说明与现状 |
|--------|------|------|
| **基础密钥** | ✅ 安全 | JWT `secret` 与 `secret_old` 平滑回转；`.env` 未入库；密码 Argon2 处理。 |
| **API 鉴权** | ✅ 安全 | 管理员路由已全量施加 `require_admin` 严格判定；用户路由有资源归属权判断。 |
| **SQL/XSS** | ✅ 安全 | 全局 sqlx 参数化查询；移动端和后端没有 HTML 渲染拼接。 |
| **脱敏策略** | ✅ 合规 | 服务器 500 报错隐藏链路栈；私有商品 Owner 字段对非所有人脱敏。 |
| **敏感资产** | ❌ 风险 | 聊天图片/语音文件保存在内网数据库，存在极高的库级别内鬼盗取或拖库泄露风险。 |
| **限流防刷** | ⚠️ 中等 | 当前限流只认 IP，遇反向代理存在全局误杀；针对聊天的限流已切换至 user_key 但未泛化至全局。 |

---

## 四、下一阶段演进路线图 (Roadmap)

我们已在 Phase 1 的第一波治理中消除了部署阻断、高危权限和连接分裂等核心问题。接下来的规划主要针对**模块化解耦**与**数据提效**。

### Phase 1 尾声：巨型文件拆解与重构（本周）
**目标**: 将系统的核心膨胀节点打散，恢复代码可读性与可维能力。
- [ ] **重构 `user_chat.rs`**：提取为 `src/api/chat/models.rs`, `connection.rs`, `message.rs` 的模块簇。
- [ ] **重构 `admin_page.dart`**：将各 Tab（统计、订单、审核）拆分至 `lib/pages/admin/tabs/`，抽象 ViewModel。
- [ ] **重构 `chat_page.dart`**：剥离录音组件与 UI 滚动处理逻辑。

### Phase 2：数据访问与存储升级（下周）
**目标**: 解决底层制约上限的严重性能包袱。
- [🔄] **对象存储剥离（进行中）**：后端已完成 URL 字段兼容迁移第一阶段（`chat_messages.image_url/audio_url`）并保持 base64 兼容读写。
- [ ] **原生 UUID**：编写 Migration，将所有 `TEXT` ID 在不中断外键约束的前提下原地 ALTER 转型为原生 `UUID`。
- [ ] **全表时间戳**：注入 `updated_at` PostgreSQL 触发器，协助增量数据同步控制。
- [ ] **事务传播模式**：改造 Repository 接口特征，使其能够接收 `&mut Transaction` 引用进行复杂跨库事务支持。

### Phase 2.1：本轮已完成的数据层兼容改造（新增）
- [x] `chat_messages` 增加 `image_url/audio_url` 列：`migrations/0013_chat_media_urls.sql`
- [x] 后端聊天模型与 SQL 双兼容（base64 + URL）：`src/api/user_chat.rs`, `src/services/chat.rs`, `src/repositories/chat_repo.rs`, `src/repositories/traits.rs`, `src/api/mod.rs`, `src/services/mod.rs`
- [x] 单测回归通过（媒体字段相关）
- [x] Flutter 聊天渲染已支持 URL 优先、base64 回退：`mobile/lib/models/models.dart`, `mobile/lib/services/chat_service.dart`, `mobile/lib/pages/user_chat_page.dart`, `mobile/lib/pages/chat_page.dart`

### Phase 2.2：本轮已完成的体验与可追踪性增强（新增）
- [x] 聊天语音播放组件（URL 优先 + base64 回退）：`mobile/lib/components/audio_message_player.dart`
- [x] 双聊天页接入语音播放：`mobile/lib/pages/user_chat_page.dart`, `mobile/lib/pages/chat_page.dart`
- [x] 数据层 `updated_at` 首批迁移：`migrations/0014_core_updated_at.sql`（users/inventory/orders + 触发器）

### Phase 2.3：本轮已完成的上传链路切流（新增）
- [x] 新增移动端上传服务（复用 STS + OSS 签名模式）：`mobile/lib/services/upload_service.dart`
- [x] 私聊语音发送已切换 URL 优先（上传成功发 `audio_url`，失败自动回退 `audio_base64`）：`mobile/lib/pages/user_chat_page.dart`
- [x] 私聊图片发送已切换 URL 优先（上传成功发 `image_url`，失败自动回退 `image_base64`）：`mobile/lib/pages/user_chat_page.dart`
- [x] 后端新增媒体路径可观测指标（URL vs base64）：`src/api/metrics.rs`, `src/api/user_chat.rs`

### Phase 2.4：本轮已完成的测试环境数据安全修复（新增）
- [x] 修复“测试误清理生产/开发库”风险：测试基础设施默认改为 `*_test` DB，并强制拦截非测试库清理（除非显式 `ALLOW_NON_TEST_DB_WIPE=1`）
- [x] 相关代码：`src/test_infra/mod.rs`

### Phase 3：工程体系现代化与治理（月底）
**目标**: 提升交付质量流护城河。
- [ ] **Provider DI 改造**：全面清退 Flutter 层面硬编码的 `new XxxService`，依靠 Provider 容器进行无损注入。
- [ ] **CI 测试体系**：搭建 GitHub Actions，执行 Docker Multi-stage 编译流体验以及核心 API 断言测试。
- [ ] **WS 背压补偿（下一步）**：在已有丢弃/清理指标基础上，引入离线消息堆积队列与短轮询降级补偿机制。

### Phase 3.1：本轮已完成的并发链路治理（新增）
- [x] WS 满载丢弃指标：`src/api/metrics.rs`
- [x] WS 僵尸连接清理指标：`src/api/metrics.rs`
- [x] WS 广播满载告警：`src/api/ws.rs`
- [x] 指标全局注册与注入：`src/main.rs`
- [ ] **动态 Embedding**：增加 `vector` 维度启动化配置系统，以便自由切换各类规模的私有 LLM 服务引擎。
