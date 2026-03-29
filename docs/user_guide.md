# Good4NCU 用户文档

> 版本：V1.0.0 RC | 更新日期：2026-03-29
> 
> **免责声明：** 本产品仅做信息发布，无担保和资金中介，也不收手续费。

---

## 第一部分：用户使用手册

### 1.1 注册与登录

**注册账号**
1. 打开 App，进入登录页面
2. 点击"注册"，输入用户名（不超过 50 字符）和密码（至少 8 字符）
3. 系统自动登录，返回 JWT Access Token（1小时有效）+ Refresh Token（7天有效）
4. Access Token 过期前，App 自动调用 `/api/auth/refresh` 续期，无需手动操作

**登录**
1. 输入用户名 + 密码，点击登录
2. 登录成功后获得 Token 对（JWT + Refresh）
3. JWT 有效期 1 小时，Refresh Token 有效期 7 天
4. Refresh Token 过期后需重新登录

**安全机制**
- 密码使用 Argon2 哈希存储，永不明文
- Refresh Token 使用 SHA-256 哈希后存储，可随时通过"退出登录"吊销
- 账号被 Admin 封禁后，所有 Token 立即失效，无法续期

---

### 1.2 浏览商品信息

**首页商品列表**
- 打开 App 即看到商品列表（分页加载，每页 20 条）
- 上拉滚动到底部自动加载更多（无限滚动）

**分类筛选**
- 支持按分类（电子产品/书籍/生活用品/服装等）筛选
- 筛选条件叠加：分类 + 排序

**搜索**
- 顶部搜索框输入关键词，支持模糊搜索商品标题/描述

**商品详情**
- 点击任意商品卡片进入详情页
- 详情包含：标题、分类、品牌、新旧程度（1-10分）、价格、描述、发布者信息
- 查看发布者信息后可发起"联系发布者"聊天

---

### 1.3 联系发布者

**发起聊天**
1. 商品详情页点击"联系发布者"
2. 系统创建连接请求，等待对方接受

**聊天连接状态**
- `pending` — 等待对方接受
- `connected` — 连接已建立，可以聊天
- `rejected` — 对方拒绝了连接请求

---

### 1.4 我的收藏

**添加收藏**
- 商品详情页点击爱心图标 → 加入收藏夹

**查看收藏夹**
- 个人中心 → 收藏列表
- 支持移除收藏

### 1.5 内容审核

**系统自动审核所有用户生成内容，违规内容将被拒绝：**

| 内容类型 | 审核范围 | 违规处理 |
|---------|---------|---------|
| 商品标题/品牌/描述 | 敏感词、外部链接 | HTTP 422 直接拒绝 |
| 聊天消息 | 敏感词、手机/微信/QQ/邮箱、外链 | HTTP 422 直接拒绝 |
| 用户名 | 敏感词 | HTTP 422 直接拒绝 |
| 头像图片 | 异步审核（阿里云 IMAN） | 延迟生效，最长数分钟 |

**常见违规内容（系统自动拦截）：**
- 敏感词 / 违禁品描述
- 手机号：`138xxxx1234`
- 微信号：`微信号 wxid_xxx`
- QQ号：`QQ 12345678`
- 邮箱地址
- 外部链接（https://...）

**头像审核说明：** 上传头像后，图片进入异步审核队列，审核通过后正式生效。在审核期间，其他用户可能暂时看到默认头像。

### 1.6 订单生命周期与自动处理

**支付超时自动取消**
- 下单后需在 **30 分钟内** 完成支付操作。
- 若超过 30 分钟未检测到支付，系统将自动取消订单（状态变为 `cancelled`）。
- **商品将自动重新上架**（状态恢复为 `active`），以便其他买家购买。

**自动收货确认**
- 卖家发货（`shipped`）后，若买家在 **7 天内** 未手动点击“确认收货”。
- 系统将自动完成订单（状态变更为 `completed`），确保交易闭环。

**状态通知**
- 订单发生自动取消或自动收货时，买卖双方都会收到 App 的 **实时通知推送** 和 **系统消息**。

---

## 第二部分：发布者使用手册

### 2.1 发布商品

**基本信息填写**
1. 个人中心 → "发布商品"
2. 填写以下字段：
   - **标题**（必填，商品名称）
   - **分类**（必填：电子产品/书籍/生活用品/服装/其他）
   - **品牌**（必填）
   - **新旧程度**（必填，1-10 分，10 分为全新）
   - **定价**（必填，单位：人民币元）
   - **商品描述**（选填，描述缺陷/使用方法等）
   - **缺陷说明**（必填，如实填写）

**AI 物品识别（推荐）**
- 可上传商品图片
- AI 自动识别分类、品牌，填充描述字段
- 发布者确认无误后提交

**发布成功**
- 商品状态为 `active`，立即可在其他用户端展示

---

### 2.2 管理我的发布

**查看我的发布**
- 个人中心 → "我的发布"
- 列表展示所有发布的商品及状态

**编辑商品**
- 点击商品 → 编辑 → 保存修改

**下架商品**
- 点击商品 → "下架"
- 下架后状态改为 `deleted`，不再展示给其他用户

---

### 2.3 处理聊天请求

**收到聊天请求通知**
- 有其他用户发起聊天时，App 首页推送通知
- 个人中心 → "聊天请求"列表

**处理方式（三选一）**
| 操作 | 结果 |
|------|------|
| 接受 | 建立聊天连接，可以与对方沟通 |
| 拒绝 | 聊天关闭，对方需重新发起 |
| 不处理 | 24-48 小时内未响应，请求自动过期 |

---

## 第三部分：Admin 后台操作手册

> **前提：** 需要 admin 角色的账号。普通用户无法访问 Admin Console。

### 3.1 访问 Admin Console

1. App 登录 admin 账号
2. 个人中心 → "Admin Console"（Tab 切页）
3. 四页签：Stats / Listings / Users

---

### 3.2 Dashboard 大盘（Stats Tab）

**数据卡片**
- **Total Listings**：全部商品数
- **Active**：在售商品数
- **Users**：注册用户总数

**趋势图（7日）**
- fl_chart 折线图展示商品发布量趋势

---

### 3.3 用户管理（Users Tab）

**浏览用户**
- 无限滚动加载下一页
- 显示：用户名 / 角色 / 注册时间 / 发布商品数

**封禁用户（Ban）**
1. 点击用户进入详情
2. 点击红色**"封禁用户 (Ban)"**按钮
3. 系统弹出 **AlertDialog 二次确认**："确定要封禁该用户吗？封禁后该用户所有登录状态将被清除。"
4. 确认后执行：
   - 用户 `status` 更新为 `banned`
   - 用户所有 Refresh Token 立即 revoke，无法续期
   - 操作记入 Audit Log：`INFO [Audit] Admin {admin_id} banned User {target_user_id} at {timestamp}`

**解封用户（Unban）**
1. 进入 banned 状态用户详情
2. 点击绿色**"解封用户 (Unban)"**按钮
3. 确认后用户恢复 `active` 状态

**安全设计**
- admin 不能封禁 admin 账号（防护）
- 封禁立即生效，无需用户重新登录

---

### 3.4 商品管理（Listings Tab）

**浏览商品**
- 无限滚动分页
- 显示：标题 / 分类 / 价格 / 状态

**强制下架（Takedown）**
1. 点击商品进入详情
2. 点击红色**"强制下架 (Takedown)"**按钮
3. 确认后执行：
   - 商品 `status` 更新为 `takedown`
   - 操作记入 Audit Log

**已下架商品**
- 状态显示为红色 "已下架" 标签
- 按钮不可点击

---

### 3.5 操作审计（Audit Logs）

**所有敏感操作均会被强制记录**：
- **封禁/解封用户**：记录操作人 ID、目标用户 ID。
- **强制下架**：记录商品 ID 及下架前后的状态。
- **修改角色**：记录权限变更详情。
- **身份模拟 (Impersonate)**：所有以他人身份登录的行为均会产生最高等级的审计。

**如何查看审计日志**
- 管理员可通过 `GET /api/admin/audit-logs` 获取结构化审计列表。
- 未来将集成至 Admin Console 的第四个标签页。

---

## 第四部分：API 参考文档

> Base URL：`http://localhost:3000`（开发环境）
> 认证方式：JWT Bearer Token（在 Header 中传递）
> 所有请求/响应 Content-Type 均为 `application/json`
> 
> **免责声明：** 本产品仅做信息发布，无担保和资金中介，也不收手续费。

---

### 4.1 认证接口

#### 注册
```
POST /api/auth/register
Body: { "username": "string", "password": "string" }
201: { "token": "jwt", "refresh_token": "uuid", "user_id": "uuid", "username": "string", "message": "注册成功" }
409: { "error": "Conflict", "message": "用户名已被使用" }
422: { "error": "ContentViolation", "message": "用户名包含违规信息，请修改后重试" }
```

#### 登录
```
POST /api/auth/login
Body: { "username": "string", "password": "string" }
200: { "token": "jwt", "refresh_token": "uuid", "user_id": "uuid", "username": "string" }
401: Unauthorized
```

#### 刷新 Token
```
POST /api/auth/refresh
Body: { "refresh_token": "string" }
200: { "token": "new_jwt", "refresh_token": "new_uuid" }
401: Refresh token 无效或已过期
```

#### 登出
```
POST /api/auth/logout
Header: Authorization: Bearer <token>
Body: { "refresh_token": "string" }（可选）
200: { "message": "已退出登录" }
```

---

### 4.2 商品接口

#### 搜索商品
```
GET /api/listings?page=1&limit=20&category=电子产品
200: { "listings": [...], "total": 100 }
```

#### 商品详情
```
GET /api/listings/:id
200: { "id", "title", "category", "brand", "condition_score", "suggested_price_cny", "description", "status", "owner_id", ... }
404: 资源不存在
```

#### 创建商品（需认证）
```
POST /api/listings
Header: Authorization: Bearer <token>
Body: { "title", "category", "brand", "condition_score", "suggested_price_cny", "description", "defects" }
201: { "id": "uuid", ... }
422: { "error": "ContentViolation", "message": "内容包含违规信息（敏感词/联系方式/外链），请修改后重试" }
```

#### 删除商品（仅 owner）
```
DELETE /api/listings/:id
Header: Authorization: Bearer <token>
204: No Content
```

---

### 4.3 聊天接口

#### 发送消息（需认证）
```
POST /api/chat
Header: Authorization: Bearer <token>
Body: { "listing_id": "uuid", "message": "string", "image_data": "base64?" }
200: { "reply": "string", "agent": "marketplace" }
422: { "error": "ContentViolation", "message": "内容包含违规信息（敏感词/联系方式/外链），请修改后重试" }
```

#### 发起聊天连接
```
POST /api/chat/connect/request
Header: Authorization: Bearer <token>
Body: { "receiver_id": "uuid", "listing_id": "uuid" }
200: { "connection_id": "uuid", "status": "pending" }
```

#### 接受聊天连接
```
POST /api/chat/connect/accept
Header: Authorization: Bearer <token>
Body: { "connection_id": "uuid" }
200: { "status": "connected" }
```

#### 拒绝聊天连接
```
POST /api/chat/connect/reject
Header: Authorization: Bearer <token>
Body: { "connection_id": "uuid" }
200: { "status": "rejected" }
```

---

### 4.4 Admin 接口（需 admin role）

#### 用户封禁
```
POST /api/admin/users/:id/ban
Header: Authorization: Bearer <token>（需 admin）
200: { "message": "用户已封禁" }
400: 用户不存在或已是 banned
403: 非 admin 无权限
```

#### 用户解封
```
POST /api/admin/users/:id/unban
Header: Authorization: Bearer <token>（需 admin）
200: { "message": "用户已解封" }
400: 用户未被封禁或不存在
```

#### 商品强制下架
```
POST /api/admin/listings/:id/takedown
Header: Authorization: Bearer <token>（需 admin）
200: { "message": "商品已强制下架" }
404: 商品不存在
```

#### 平台统计
```
GET /api/admin/stats
Header: Authorization: Bearer <token>（需 admin）
200: { 
  "total_listings": 100, 
  "active_listings": 80, 
  "total_users": 50, 
  "total_orders": 20, 
  "admin_users": 2,
  "categories": [ { "category": "书籍", "count": 10 }, ... ]
}
```

#### 审计日志查询
```
GET /api/admin/audit-logs?limit=50&offset=0
Header: Authorization: Bearer <token>（需 admin）
200: {
  "total": 120,
  "logs": [
    {
      "id": "uuid",
      "admin_id": "uuid",
      "action": "ban_user",
      "target_id": "user-123",
      "created_at": "ISO-8601"
    },
    ...
  ]
}
```

---

## 第五部分：部署与运维

### 5.1 环境变量

| 变量名 | 必填 | 默认值 | 说明 |
|--------|------|--------|------|
| `DATABASE_URL` | Yes | — | PostgreSQL 连接串，格式：`postgres://user:pass@host:5432/db` |
| `JWT_SECRET` | Yes | — | JWT 签名密钥，建议 32+ 随机字符 |
| `GEMINI_API_KEY` | Yes* | — | Google Gemini API Key（`*LLM_PROVIDER=gemini` 时必填） |
| `LLM_PROVIDER` | No | `gemini` | LLM 提供商：`gemini` 或 `minimax` |
| `MINIMAX_API_KEY` | No | — | MiniMax API Key（`LLM_PROVIDER=minimax` 时必填） |
| `MINIMAX_API_BASE_URL` | No | — | MiniMax API Base URL |
| `RATE_LIMIT_MAX_REQUESTS` | No | `100` | 每时间窗口最大请求数 |
| `RATE_LIMIT_WINDOW_SECS` | No | `60` | 限流时间窗口（秒） |
| `VECTOR_DIM` | No | `768` | 向量维度（pgvector embedding） |
| `CORS_ORIGINS` | No | `*` | 允许的跨域源，多个用逗号分隔 |
| `BLOCKED_KEYWORDS` | No | — | 逗号分隔的敏感词列表 |
| `MODERATION_IMAGE_API_KEY` | No | — | 阿里云 IMAN 图片审核 API Key |
| `MODERATION_IMAGE_API_URL` | No | — | 阿里云 IMAN API 端点 |

### 5.2 Docker 部署

**构建镜像**
```bash
docker build -t good4ncu:latest .
```

**docker-compose.yaml 示例**
```yaml
version: '3.8'
services:
  app:
    image: good4ncu:latest
    ports:
      - "3000:3000"
    environment:
      DATABASE_URL: postgres://postgres:password@db:5432/good4ncu
      JWT_SECRET: your-32-char-secret-key-here
      GEMINI_API_KEY: your-gemini-api-key
      LLM_PROVIDER: gemini
    depends_on:
      db:
        condition: service_healthy

  db:
    image: pgvector/pgvector:pg16
    environment:
      POSTGRES_PASSWORD: password
      POSTGRES_DB: good4ncu
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5

volumes:
  pgdata:
```

**启动**
```bash
docker-compose up -d
```

**数据库初始化**
- 应用启动时自动执行 `sqlx::migrate!()`，运行 `migrations/` 下所有 migration
- 无需手动运行 SQL

### 5.3 数据库 Schema

**核心表**

| 表名 | 说明 |
|------|------|
| `users` | 用户（id, username, password_hash, role, **status**) |
| `inventory` | 商品（id, title, category, brand, condition_score, price, status, owner_id） |
| `chat_messages` | 聊天消息（id, conversation_id, sender, content, timestamp） |
| `chat_connections` | 聊天连接（requester, receiver, status） |
| `refresh_tokens` | Refresh Token（user_id, token_hash, expires_at, revoked_at） |
| `notifications` | 通知（user_id, event_type, title, body, is_read） |
| `watchlist` | 收藏夹（user_id, listing_id） |
| `documents` | pgvector RAG 向量文档（id, document, embedding） |

**重要索引**
```sql
CREATE INDEX idx_users_status ON users(status);
CREATE INDEX idx_chat_conversation ON chat_messages(conversation_id, timestamp);
CREATE INDEX document_embeddings_idx ON documents USING hnsw(embedding vector_cosine_ops);
```

### 5.4 可观测性

**Prometheus Metrics**
```
GET /api/metrics
```
返回 Prometheus Text Exposition 格式，指标包括：

| 指标名 | 类型 | 说明 |
|--------|------|------|
| `http_requests_total` | CounterVec | HTTP 请求总数（method, path, status） |
| `http_request_duration_seconds` | HistogramVec | HTTP 请求延迟 |
| `chat_messages_total` | Counter | 聊天消息数 |
| `rate_limit_rejected_total` | Counter | 限流拒绝数 |
| `llm_calls_total` | Counter | LLM 调用成功数 |
| `llm_errors_total` | Counter | LLM 调用失败数 |

**结构化日志**
- 应用日志格式：JSON（带 `target` + `thread_id`）
- 日志级别：ERROR / WARN / INFO / DEBUG
- Admin 操作 Audit Log：`INFO [Audit] Admin {admin_id} {action} {target} at {timestamp}`

### 5.5 安全防护

| 防护项 | 实现 |
|--------|------|
| JWT 签名 | HS256 + 密钥验签 |
| 密码存储 | Argon2 哈希 |
| Refresh Token | UUID v4，SHA-256 哈希存储，7天有效期 |
| 权限控制 | `require_admin()` 中间件，role=admin 验证 |
| 限流 | Token Bucket，20 req/min per IP（`/api/chat`） |
| 内容审核 | 文本同步审核（敏感词/联系方式/外链）；图片异步审核（IMAN stub） |
| SQL 注入 | 完全使用 sqlx 参数化查询 |
| Admin 审计 | 所有写操作记 Audit Log |

### 5.6 熔断降级

**LLM Circuit Breaker**
- 5 次连续失败 → 熔断打开，Fail Fast
- 30 秒后半开探测
- 成功 → 闭合恢复正常

**降级响应示例**
```json
{
  "reply": "抱歉，AI服务暂时不可用，请稍后再试或联系客服。"
}
```

---

## 第六部分：免责声明

**本产品仅做信息发布，无担保和资金中介，也不收手续费。**

具体说明：
- 本平台仅提供信息发布和搜索服务，不参与任何交易过程
- 平台不对交易真实性、商品质量、资金安全承担任何担保责任
- 用户之间私下交易产生的任何纠纷与本平台无关
- 本平台不收取任何手续费或佣金

如有任何疑问，请联系平台客服。
