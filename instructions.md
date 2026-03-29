# P10 工程执行指令 (Sprint 4 - Polish & Stabilization)

## ⚡ 已由 P10 亲自修复的问题

### 聊天功能四连杀 — 4 个 BUG 已全部闭环

P10 亲自排查后发现聊天功能不是"一个 BUG"，而是 4 个独立 BUG 叠加导致的系统性协议不一致。以下均已修复：

| # | BUG | 根因 | 修复文件 | 状态 |
|---|-----|------|---------|------|
| 1 | **WS 实时推送全部静默失效** | Backend 发 `"event"` 字段，Frontend 读 `"event_type"` | `ws_service.dart:43` | ✅ |
| 2 | **发消息后出现幽灵消息** | `send_connection_message` 返回 4 个字段，Frontend 需要 9 个 | `user_chat.rs` SendMessageResponse | ✅ |
| 3 | **进入聊天页面触发 N+1 请求风暴** | `markConnectionAsRead` 逐条 HTTP 标记，50 未读 = 51 次请求 | 新增 `POST /api/chat/connection/{id}/read` 批量接口 | ✅ |
| 4 | **Profile 页面永远加载中** | `initState` 内同步调用 `AppLocalizations.of(context)!` | `profile_page.dart` | ✅ (前次已修) |

**修改文件清单：**
- `mobile/lib/services/ws_service.dart` — `json['event'] ?? json['event_type']` 兼容
- `src/api/user_chat.rs` — `SendMessageResponse` 补全字段 + 新增 `mark_connection_read` handler
- `src/api/mod.rs` — 注册 `/api/chat/connection/{id}/read` 路由
- `mobile/lib/services/api_service.dart` — `markConnectionAsRead` 改为单次 POST

---

## 待手下完成的任务（可并行）

### 任务 1：修复 Admin Console Orders 操作报错 (Fullstack P1)

**现象：** Admin 面板 Orders Tab 下点击 "Cancel" / "Confirm" 报 403；订单金额显示放大 100 倍。

**根因：**
1. `admin_page.dart` 的 `_showOrderDetail` 误调了 `cancelOrder()` / `confirmOrder()`，这些是买卖双方专属接口，Admin 身份会被 403。
2. 后端 `get_admin_orders` 返回原始 `final_price`（分币制 i64），前端直接当"元"渲染。

**执行步骤：**
1. `api_service.dart` 新增 `updateAdminOrderStatus(String orderId, String status)` → `POST /api/admin/orders/$orderId/status`，body: `{"status": status}`。
2. `admin_page.dart` `_showOrderDetail` 中 Cancel 按钮改调 `updateAdminOrderStatus(id, 'cancelled')`；Confirm 按钮改调 `updateAdminOrderStatus(id, 'completed')`。
3. Orders 列表显示金额处，`item['final_price']` 除以 100 后展示（或者后端 `admin.rs` 用 `cents_to_yuan` 转换后再返回）。

### 任务 2：实现 Admin Impersonate 功能 (Frontend P1)

**现象：** 后端已有 `POST /api/admin/users/:id/impersonate`，前端无入口。

**执行步骤：**
1. `api_service.dart` 新增 `impersonateUser(String userId)` 调用该接口，返回完整 response。
2. `admin_page.dart` `_showUserDetail()` 底部新增 "模拟登录" 按钮（紫色/警示色，仅对非 banned、非 admin 用户显示）。
3. 点击时：弹出确认对话框 → 调用 `impersonateUser` → 将返回的 `token` 覆盖 SharedPreferences 中 `jwt_token`，清空 `refresh_token` → 强制路由到首页。

---

请各位研发同学认领以上工作，编译验证通过后提交。聊天功能的核心 BUG 我已经全部亲自修完了。

### 任务 3：修复 Login 401 报错文案 (由 P10 亲自修复)

**现象：** 用户输入错误的密码或用户名时，系统提示："请先登录后再操作" (401 Unauthorized)，导致前端无法正确给用户提示 "用户名或密码错误"，P9 排查陷入死胡同。

**根因：**
在 `src/api/error.rs` 中，`ApiError::Unauthorized` 被全局硬编码映射为了 `(StatusCode::UNAUTHORIZED, "请先登录后再操作")`。而 `src/api/auth.rs` 中密码错误、用户找不到时，统一抛出了 `ApiError::Unauthorized`，导致了全局拦截冲突。

**执行步骤 (已完成)：**
1. 在 `src/api/error.rs` 中新增了 `ApiError::AuthFailed(String)` 变体，映射为 `(StatusCode::UNAUTHORIZED, "认证失败: {0}")`。
2. 在 `src/api/auth.rs` 中，将对应的 `Err(ApiError::Unauthorized)` 替换为了 `Err(ApiError::AuthFailed("用户名或密码错误".to_string()))`（针对 `login` 接口），以及修改密码时的 `AuthFailed("当前密码错误")`。
3. Session 失效等场景仍保持使用 `ApiError::Unauthorized` 以产生 "请先登录后再操作"。

👉 **请后端同学拉取最新代码并重启服务 (`cargo run`)，即可生效。**
