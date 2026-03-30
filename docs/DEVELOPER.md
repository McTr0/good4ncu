# DEVELOPER.md — good4ncu 开发指南

> 面向贡献者的开发流程、架构决策、多Agent团队协作规范。
>
> **免责声明：** 本产品仅做信息发布，无担保和资金中介，也不收手续费。

---

## 目录

- [团队协作模式](#团队协作模式)
- [Agent团队与职责](#agent团队与职责)
- [开发环境搭建](#开发环境搭建)
- [代码审查流程](#代码审查流程)
- [测试策略](#测试策略)
- [架构决策记录](#架构决策记录)
- [Flutter开发指南](#flutter开发指南)
- [Rust后端开发指南](#rust后端开发指南)
- [后台 Worker 开发指南](#后台-worker-开发指南)
- [安全编码规范（SQL与审计）](#安全编码规范sql与审计)

---

## 团队协作模式

### Agent团队工作流

本项目采用 **multi-agent编排** 模式，多个专业Agent各司其职，覆盖从设计到实现的完整生命周期：

```
用户请求
    │
    ▼
┌─────────────────┐
│  architect      │ ← 复杂功能 / 重构 / 新模块 → 输出设计文档
└────────┬────────┘
         │ 批准后
         ▼
┌─────────────────┐
│  planner        │ ← 实施计划 → 分解任务清单
└────────┬────────┘
         │
    ┌────┼────┬────────┐
    ▼    ▼    ▼        ▼
┌──────┐ ┌────┐ ┌──────┐ ┌────────────┐
│ rust │ │rust│ │flutter│ │security    │
│build │ │review│ │reviewer│ │reviewer    │
│resolver│ │er │ │       │ │            │
└──────┘ └──┬─┘ └──────┘ └─────┬──────┘
             │                   │
             ▼                   ▼
        ┌─────────────────────────┐
        │     code-reviewer       │ ← 所有代码变更必须经过
        └────────────┬────────────┘
                     │ 通过后
                     ▼
              ┌────────────┐
              │  提交代码  │
              └────────────┘
```

### Agent触发规则

| 场景 | Agent | 原因 |
|------|-------|------|
| 新功能开发 | `planner` | 需要分解任务、识别依赖 |
| 复杂架构决策 | `architect` | 系统设计需要多方权衡 |
| Rust代码变更 | `rust-reviewer` | 所有权/lifetime/安全性审查 |
| Flutter代码变更 | `flutter-reviewer` | Widget最佳实践/性能审查 |
| Bug修复 | `tdd-guide` | 先写测试，确保复现 |
| 安全相关代码 | `security-reviewer` | 主动防护OWASP Top 10 |
| 构建失败 | `build-error-resolver` | 快速定位编译/类型错误 |
| 集成测试/E2E | `e2e-runner` | 端到端验证关键流程 |
| 死代码清理 | `refactor-cleaner` | 主动维护代码质量 |
| 文档更新 | `doc-updater` | 同步文档与实现 |

---

## Agent团队与职责

### architect — 系统架构师

**职责：** 复杂功能设计、架构决策、技术选型评估

**触发场景：**
- 新增微服务或独立模块
- 重构影响核心数据流
- 跨语言/跨框架集成决策
- 性能敏感场景的设计选择

**输出：** 架构设计文档，包含权衡分析、替代方案对比、风险评估

```markdown
# 架构提案: [功能名称]

## Context（背景）
[为什么需要这个改动]

## Decision（决策）
[具体方案]

## Consequences（影响）
- 正面：[收益]
- 负面：[ Trade-off ]
```

---

### planner — 实施计划专家

**职责：** 分解任务、识别依赖、建立实施路径

**触发场景：**
- Feature issue被批准后
- 需要多文件/多模块协同的任务
- 需要数据库迁移的功能
- 需要前后端协同的功能

**工作方式：**
1. 理解需求范围和验收标准
2. 识别文件级依赖和执行顺序
3. 标记需要review的关键路径
4. 输出可执行的任务清单

---

### rust-reviewer — Rust代码审查专家

**职责：** 所有权、生命周期、并发安全、错误处理、惯用Rust代码

**审查清单：**
- [ ] `Clone`派生是否必要（避免不必要的复制）
- [ ] `Arc`/`Mutex`使用是否正确（共享状态）
- [ ] 生命周期标注是否合理（避免过度约束）
- [ ] `async`/`await`模式是否正确（`!Send`future处理）
- [ ] Error层级是否清晰（`thiserror` vs `anyhow`）
- [ ] 不会在`unsafe`块中引入未定义行为
- [ ] 符合[ Rust编码规范](../.claude/rules/rust/coding-style.md)

**常见问题：**
```rust
// ❌ 过度约束的生命周期
fn foo<'a: 'b, 'b>(x: &'a i32, y: &'b i32) {}

// ✅ 最小约束
fn foo(x: &i32, y: &i32) {}

// ❌ 不必要的Clone
let data = data.clone();

// ✅ 考虑所有权转移或引用
fn process(data: &Data) { ... }
```

---

### flutter-reviewer — Flutter/Dart代码审查专家

**职责：** Widget最佳实践、状态管理模式、性能优化、可访问性

**审查清单：**
- [ ] 无状态Widget被正确使用（`const`构造）
- [ ] 状态管理符合项目模式（`Provider`）
- [ ] 异步操作后正确处理`context`（lint: `use_build_context_synchronously`）
- [ ] 列表渲染使用了`ListView.builder`等高效模式
- [ ] 资源泄漏检查（`dispose()`实现、Stream订阅取消）
- [ ] 国际化字符串使用`AppLocalizations`（非硬编码文本）
- [ ] 符合[ Flutter最佳实践](https://docs.flutter.dev/perfs)

**常见问题：**
```dart
// ❌ 每次build重新创建
ListView(
  children: List.generate(100, (i) => MyWidget()),
)

// ✅ 使用builder模式
ListView.builder(
  itemCount: 100,
  itemBuilder: (context, i) => MyWidget(),
)

// ❌ 异步后使用已卸载的context
FutureBuilder(
  future: fetchData(),
  builder: (context, snapshot) {
    await Future.delayed(...); // ❌ 错误
  },
)
```

---

### security-reviewer — 安全审查专家

**职责：** 主动识别安全漏洞、密钥管理、注入攻击防护

**审查清单：**
- [ ] 无硬编码密钥（API key、JWT secret、密码）
- [ ] 所有用户输入经过验证
- [ ] SQL使用参数化查询（无字符串拼接）
- [ ] 文件上传验证类型和大小
- [ ] 认证/授权正确实现
- [ ] 错误消息不泄漏敏感信息
- [ ] Rate limiting覆盖所有公开端点
- [ ] CORS配置正确（生产环境）

**OWASP Top 10 重点：**
- A01 失效的访问控制
- A02 加密失败（密钥存储、传输加密）
- A03 注入（SQL、命令注入）
- A04 不安全的设计
- A05 安全配置错误
- A06 易受攻击的组件
- A07 认证失败
- A08 数据完整性失败
- A09 日志和监控不足
- A10 服务器请求伪造（SSRF）

---

### code-reviewer — 综合代码审查

**职责：** 通用代码质量、一致性、可维护性

**审查维度：**
- 代码可读性（命名、注释、函数长度）
- 错误处理完整性
- 边界条件处理
- 性能考虑（数据库查询效率、内存使用）
- 符合项目编码规范

---

### build-error-resolver — 构建错误解决专家

**职责：** 快速定位和修复编译错误、类型错误、Lint警告

**触发场景：**
- `cargo build` / `cargo check` 失败
- `cargo clippy` 报告错误
- `flutter analyze` 报告错误
- 依赖版本冲突

**方法：**
1. 分析错误信息（首行通常指向真正问题）
2. 检查最近的依赖变更（`Cargo.lock` / `pubspec.lock`）
3. 增量排查（`--offline`模式快速验证依赖解析）
4. 修复后验证（`cargo check && cargo clippy`）

---

### tdd-guide — 测试驱动开发专家

**职责：** 指导编写测试、确保测试覆盖率、维护测试套件

**TDD流程：**
```
RED    → 写一个失败的测试（明确期望行为）
GREEN  → 写最小实现让测试通过
REFACTOR → 重构代码，测试保持通过
```

**覆盖率要求：**
- 业务逻辑：≥80%
- Repository层：≥70%
- API handler：≥60%
- 前端widget：关键路径覆盖

---

### e2e-runner — 端到端测试专家

**职责：** 关键用户流程的E2E测试、测试用例设计

**关键流程覆盖：**
- 用户注册 → 登录 → 发布商品 → 聊天 → 下单
- 管理员：登录 → 审核商品 → 封禁用户
- AI助手：商品咨询 → 议价引导

---

### doc-updater — 文档更新专家

**职责：** 保持文档与代码实现同步，确保文档准确反映系统架构和使用方法

**触发场景：**
- 新功能开发完成后
- API端点变更
- 架构决策记录（ADR）更新
- 配置格式变更
- 数据库Schema变更

**工作方式：**
1. 提取代码中的JSDoc/TSDoc注释
2. 同步更新相关Markdown文档
3. 验证文档中的示例能正确运行
4. 更新API参考文档

---

## 开发环境搭建

### Rust后端

```bash
# 1. 安装Rust（如果还没有）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update

# 2. 克隆项目
git clone https://github.com/your-org/good4ncu.git
cd good4ncu

# 3. 创建.env文件
cp .env.example .env
# 编辑.env填入实际值

# 4. 启动PostgreSQL（需要pgvector扩展）
psql $DATABASE_URL -c "CREATE EXTENSION IF NOT EXISTS vector;"

# 5. 运行迁移（首次启动时sqlx自动创建）
cargo run

# 6. 验证
# 浏览器打开 http://127.0.0.1:3000
# 或 curl http://127.0.0.1:3000/api/health
```

### Flutter移动端

```bash
# 1. 安装Flutter SDK
# https://docs.flutter.dev/get-started/install

# 2. 检查环境
flutter doctor

# 3. 安装依赖
cd mobile
flutter pub get

# 4. 运行（开发模式，连接本地后端）
flutter run

# 5. 指定后端地址
flutter run --dart-define=API_BASE_URL=http://YOUR_IP:3000
```

### 配置说明

**环境变量优先级：** `env > TOML文件 > hardcoded默认值`

```bash
# 必需的环境变量
DATABASE_URL=postgres://user:pass@localhost:5432/good4ncu
GEMINI_API_KEY=your_gemini_api_key   # 或 MINIMAX_API_KEY
JWT_SECRET=your_jwt_secret_at_least_32_chars

# 可选（可通过good4ncu.toml配置）
LLM_PROVIDER=gemini  # 或 minimax
VECTOR_DIM=768
SERVER_PORT=3000
```

TOML配置示例：

```toml
[llm]
provider = "gemini"
vector_dim = 768

[server]
host = "0.0.0.0"
port = 3000

[moderation]
blocked_keywords = "admin,manager,支付宝,微信,QQ"
image_enabled = true
image_api_url = "https://api.imantect.example.com/v1/moderation"
image_api_key = "your-image-moderation-api-key"
image_max_retries = 3
```

**TOML配置文件搜索路径：**
1. `$CONFIG_FILE` 环境变量指定路径
2. `./good4ncu.toml`（项目根目录）
3. `./config/good4ncu.toml`（配置目录）

详见 [config.toml.example](config.toml.example)

---

## 代码审查流程

### Pull Request Checklist

在创建PR前，确保以下所有项通过：

```bash
# Rust后端
cargo fmt
cargo clippy -- -D warnings
cargo test
# 或快速验证
cargo check

# Flutter前端
cd mobile
flutter analyze
flutter test
```

### Git提交规范

使用 Conventional Commits：

```
feat: 添加用户邮箱修改功能
fix: 修复商品列表分页问题
refactor: 重构Repository层提取公共trait
docs: 更新README快速启动章节
test: 添加订单创建集成测试
chore: 升级Gemini SDK版本
perf: 优化商品搜索查询效率
ci: 添加GitHub Actions工作流
```

### 分支管理

```
master          ← 主分支，生产就绪
├── develop     ← 开发集成分支
│   ├── feat/user-email     ← 功能分支
│   ├── fix/chat-reconnect  ← Bug修复分支
│   └── refactor/repo-layer ← 重构分支
└── ...
```

1. 从`master`创建功能分支
2. 开发完成后发起PR到`master`
3. 至少1人review通过后合并
4. 使用`git rebase -i`整理提交历史（避免"wip"、"fix"等无用提交）

---

## 测试策略

### 单元测试

```bash
# Rust
cargo test --lib

# Flutter
cd mobile && flutter test
```

### 集成测试

```bash
# Rust（需要真实数据库）
TEST_DATABASE_URL="postgres://mctr0@localhost/good4ncu_test" \
DATABASE_URL="postgres://mctr0@localhost/good4ncu" \
  cargo test --test product_integration -- --test-threads=1
```

> ⚠️ 安全说明：所有会执行清理数据的测试必须连接 `*_test` 库。
> 框架已默认拦截非测试库清理；仅在你明确知道风险时才可设置 `ALLOW_NON_TEST_DB_WIPE=1` 覆盖。

### E2E测试

```bash
# Flutter E2E
cd mobile
flutter test integration_test/
```

### 测试覆盖

| 层级 | 目标覆盖率 | 重点 |
|------|-----------|------|
| Service层 | ≥80% | 业务逻辑、分支路径 |
| Repository层 | ≥70% | 查询构造、错误处理 |
| API Handler | ≥60% | 请求验证、响应格式 |
| Widget | 关键路径 | 用户交互流程 |

---

## 架构决策记录

### ADR-001: 使用Repository模式分离数据访问

**Context:** API handler直接调用sqlx导致数据访问逻辑散落各处，难以测试和替换存储后端。

**Decision:** 在`src/repositories/`中定义trait，API handler依赖抽象接口而非具体实现。

**Consequences:**
- ✅ 便于Mock测试（测试时可注入内存实现）
- ✅ 存储后端可替换（PostgreSQL → 其它DB）
- ❌ 增加一层间接调用

**状态：** 已实施（Phase 1 完成）

---

### ADR-002: 使用AppState分组消除God Object

**Context:** AppState有17个字段，新成员难以理解字段职责分组。

**Decision:** 将AppState拆分为`ApiSecrets`（静态配置）、`ApiInfrastructure`（运行时）、`ApiAgents`（LLM）。

**Consequences:**
- ✅ 零breaking change（handler访问路径从`state.xxx`改为`state.group.xxx`）
- ✅ IDE自动提示更清晰
- ❌ 需要更新所有handler中的字段访问

**状态：** 已实施（Phase 2 完成）

---

### ADR-003: Flutter按领域拆分Service

**Context:** ApiService是818行47方法的God Class。

**Decision:** 按领域拆分为`AuthService`、`ListingService`、`ChatService`等，引入Provider依赖注入。

**Consequences:**
- ✅ 单个Service文件≤200行
- ✅ 便于独立测试
- ✅ 新成员可快速定位相关代码

**状态：** 已实施（Phase 3 完成）

---

### ADR-004: 引入分布式管理员审计追踪（Audit Logging）

**Context:** 管理员拥有极高权限（封禁、冒充登录、强制下架），若缺乏操作追踪，将产生巨大的安全与内部合规风险。

**Decision:** 建立 `admin_audit_logs` 表，并在 `AdminService` 中统一处理审计逻辑。所有 `admin.rs` 中的写操作必须调用 `log_action`。

**Consequences:**
- ✅ **不可抵赖性**：所有敏感操作均有时间戳、操作人、目标对象及变更详情。
- ✅ **辅助复原**：记录了 `old_value` 和 `new_value`，便于在误操作后进行手动数据恢复。
- ❌ 增加了一次数据库 I/O 开销（由于是后台管理操作，性能影响可忽略）。

**状态：** 已实施（Phase 4）

---

### ADR-005: 订单状态机自动化（Order Lifecycle Workers）

**Context:** 订单存在支付时效（30分钟）和收货确认时效（7天），依赖人工点击或前端逻辑是不靠谱的。

**Decision:** 采用 `tokio::spawn` 启动独立后台协程，以 5 分钟为步长轮询数据库，通过 `FOR UPDATE SKIP LOCKED` 实现简单的多实例竞争避让。

**Consequences:**
- ✅ **资源及时释放**：未支付订单自动取消，关联商品自动恢复 `active` 状态。
- ✅ **自动化结算**：到期自动收货，保障卖家资金周转。
- ❌ 增加了数据库定期轮询的压力（可通过优化索引减轻）。

**状态：** 已实施（Phase 4）

---

### ADR-006: 内容审核架构（Content Moderation）

**Context（背景）:**

聊天系统和商品列表中用户生成内容（UGC）需要内容审核，防止：
- 敏感词（政治、色情、违禁品等）
- 联系方式（手机号、微信号、QQ号、支付宝等）
- 外部链接（诱导用户到其他平台）

**Decision（决策）:**

采用**同步文本审核 + 异步图片审核**的混合架构：

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Content Moderation Architecture                 │
├─────────────────────────────────────────────────────────────────────┤
│  [User Content]                                                      │
│       │                                                              │
│       ├─── Text Content ────► [check_text()] ────► HTTP 422          │
│       │                        │                   (rejected)        │
│       │                        ▼                                      │
│       │                   [passed]                                   │
│       │                        │                                      │
│       │                        ▼                                      │
│       │                   [Persist]                                  │
│       │                                                              │
│       └─── Image Content ──► [submit_image_job()] ──► job_id        │
│                                │                     (async)          │
│                                ▼                                      │
│                         [moderation_jobs table]                      │
│                                │                                      │
│                                ▼                                      │
│                    [ModerationWorker] ◄─── polling                   │
│                                │                                      │
│                                ▼                                      │
│                    [IMAN API / Stub]                                 │
│                                │                                      │
│                                ▼                                      │
│                    [update_resource_status()]                        │
└─────────────────────────────────────────────────────────────────────┘
```

**ModerationService API:**

```rust
// 文本审核（同步）
pub async fn check_text(&self, content: &str) -> Result<ModerationResult, ModerationError>

// 图片审核任务提交（异步，立即返回）
pub async fn submit_image_job(
    &self,
    resource_type: &str,  // "listing_image", "chat_image"
    resource_id: &str,
    image_data: &[u8],
) -> Result<String, ModerationError>  // Returns job_id

// ModerationCode 枚举
pub enum ModerationCode {
    Pass,           // 通过
    Profanity,      // 脏话/敏感词
    ContactInfo,    // 联系方式（手机/微信/QQ/支付宝）
    ExternalLink,   // 外部链接
    Spam,           // 垃圾信息
    Political,      // 政治敏感
    Adult,          // 色情内容
    Other(String),  // 其他原因
}
```

**ModerationWorker:**

```rust
pub fn run_moderation_worker(
    pool: PgPool,
    moderation_service: Arc<ModerationService>,
    shutdown_rx: oneshot::Receiver<()>,
) -> JoinHandle<()>
```

CRITICAL: `moderation_worker_handle.abort()` must be called on shutdown.

**状态：** 已实施

---

## Flutter开发指南

### 状态管理

项目使用`Provider`进行状态管理：

```dart
// 定义Provider
final apiServiceProvider = Provider<ApiService>((ref) => ApiService());
final authServiceProvider = Provider<AuthService>((ref) {
  return AuthService(ref.watch(apiServiceProvider));
});

// 在页面中使用
class MyPage extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final authService = ref.watch(authServiceProvider);
    // ...
  }
}
```

### API调用规范

```dart
// ✅ 正确：捕获ScaffoldMessenger在await之前
Future<void> _submit() async {
  final messenger = ScaffoldMessenger.of(context);
  final result = await apiService.submit(data);
  if (mounted) {
    messenger.showSnackBar(SnackBar(content: Text(result.message)));
  }
}

// ❌ 错误：await后使用context
Future<void> _submit() async {
  await apiService.submit(data);
  ScaffoldMessenger.of(context).showSnackBar(...); // lint警告！
}
```

### 国际化

所有用户可见文本必须使用`AppLocalizations`，禁止硬编码：

```dart
// ✅
Text(l.cancel)

// ❌
Text('取消')
```

新增key：
1. 编辑`mobile/lib/l10n/app_en.arb`（英文）
2. 编辑`mobile/lib/l10n/app_zh.arb`（中文）
3. 运行`flutter gen-l10n`生成代码

---

## Rust后端开发指南

### 模块导入顺序

```rust
use std::...;           // 标准库
use tokio::...;         // 外部crate（按字母）
use axum::...;
use sqlx::...;

use crate::...;         // 内部crate（相对于当前位置）
use super::...;
```

### 错误处理模式

```rust
// 库/领域错误 → thiserror
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("listing not found: {0}")]
    NotFound(String),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

// 应用级错误 → anyhow
fn process() -> anyhow::Result<()> {
    let data = read_config()?;
    validate(&data)?;
    Ok(())
}
```

### 数据库操作

```rust
// ✅ 参数化查询（防注入）- 强烈推荐
let row = sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE id = $1",
    user_id
)
.fetch_one(&pool)
.await?;

// ✅ 使用 sqlx::query().bind() - 动态构建 SQL 时使用
let query = "UPDATE orders SET status = $1 WHERE id = $2";
sqlx::query(query)
    .bind(new_status)
    .bind(order_id)
    .execute(&pool)
    .await?;

// ❌ 严禁使用 format! 拼接用户输入或 ID
let query = format!("SELECT * FROM users WHERE id = '{}'", user_id);
```

### ModerationService 内容审核服务

所有用户生成的文本内容在保存前必须经过审核：

```rust
// 文本审核（同步）
// 失败返回 ModerationError::ContentViolation
pub async fn check_text(&self, content: &str) -> Result<ModerationResult, ModerationError>

// 提交图片审核任务（异步）
pub async fn submit_image_job(
    &self,
    resource_type: &str,   // "listing_image" | "chat_image" | "avatar"
    resource_id: &str,
    image_data: &[u8],
) -> Result<String, ModerationError>  // job_id
```

**图片审核状态枚举 (ImageModerationStatus):**

```rust
pub enum ImageModerationStatus {
    Approved,   // 图片审核通过
    Pending,    // 待审核
    Rejected,   // 图片审核未通过
}
```

**图片审核任务结构 (ImageModerationJob):**

```rust
pub struct ImageModerationJob {
    pub id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub image_url: String,
    pub status: String,
    pub retry_count: i32,
}
```

### 后台 Worker 开发指南

项目在 `src/services/` 下维护了多个自动化 Worker：

1. **`hitl_expire.rs`**: 处理议价请求超时（48小时）。
2. **`order_worker.rs`**:
   - **支付超时**：`pending` -> `cancelled` (30m)。逻辑：取消订单 + 恢复商品 Inventory 为 `active`。
   - **自动完成**：`shipped` -> `completed` (7d)。
3. **`moderation_worker.rs`**: 异步图片审核 Worker，轮询 `moderation_jobs` 表，调用阿里云 IMAN API。
   - **关键**：必须在 `main.rs` 的 shutdown 序列中调用 `moderation_worker_handle.abort()`。

**开发要点：**
- **幂等性**：Worker 逻辑必须支持无限次重复运行而不产生副作用。
- **批量处理**：使用 `fetch_all` 配合 `FOR UPDATE SKIP LOCKED` 提高扫描效率。
- **通知触发**：Worker 在变更状态后，必须调用 `broadcast` 推送 WebSocket 通知。

### 安全编码规范（SQL与审计）

- **参数化查询**：除表名/字段名等静态标识符外，所有动态变量必须通过 `.bind()` 传递。
- **敏感操作审计**：凡涉及 `api/admin.rs` 中的 Handler，必须显式调用 `infra.admin_service.log_action`。
- **软删除原则**：商品和订单记录不应物理删除，统一使用 `status = 'deleted' / 'cancelled'` 进行逻辑标记。

### 异步Runtime

```rust
// ✅ 使用tokio::spawn处理后台任务
tokio::spawn(async move {
    // 独立生命周期
    process_event(event).await;
});

// ✅ 事件总线发送（fire-and-forget）
let _ = tx.send(BusinessEvent::ChatMessage(msg));
```

---

## 常见问题

### Q: 数据库迁移如何处理？

A: 项目使用sqlx的`sqlx migrate`命令管理迁移脚本，存放在`migrations/`目录，按序号命名（`0001__xxx.sql`）。首次连接数据库时sqlx会自动执行所有迁移。

### Q: 如何添加新的API端点？

**Rust:**
1. 在对应模块文件中添加handler函数
2. 在`src/api/mod.rs`中注册路由
3. 添加单元测试
4. 更新API文档（如有）

**Flutter:**
1. 在对应Service中添加方法（如`ListingService`）
2. 调用`ApiService`的HTTP方法
3. 添加Widget测试（如有必要）

### Q: LLM提供商如何切换？

A: 设置`LLM_PROVIDER=gemini`或`LLM_PROVIDER=minimax`。代码中`LlmProvider` trait抽象了所有LLM操作，无需修改业务逻辑。

### Q: 前端如何连接后端？

Flutter默认连接`http://127.0.0.1:3000`。生产环境部署时使用：
```bash
flutter run --dart-define=API_BASE_URL=https://your-backend-domain.com
```

---

## 相关文档

- [README.md](README.md) — 用户文档、产品特性、快速启动
- [CLAUDE.md](CLAUDE.md) — Claude Code AI助手指南、项目架构概述
- [AGENTS.md](AGENTS.md) — Agent框架详细文档
- [config.toml.example](config.toml.example) — 配置文件参考
