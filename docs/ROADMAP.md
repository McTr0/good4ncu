# good4ncu Roadmap — Pending Items & Implementation Plan

> Last updated: 2026-03-29
> Status: Active development

---

## Overview

This document catalogs all DISABLED, placeholder, and stub functionality in the good4ncu monorepo, with implementation plans ordered by priority and dependency.

---

## Priority Phases

### Phase 0 — 内容审核系统 (Content Moderation) ✅ 已完成

| # | Item | Status | Files |
|---|------|--------|-------|
| 1 | 文本审核（敏感词/手机/微信/QQ/邮箱/外链 → HTTP 422） | ✅ | `src/services/moderation.rs` |
| 2 | 图片审核 Worker（异步轮询 + FOR UPDATE SKIP LOCKED） | ✅ | `src/services/moderation_worker.rs` |
| 3 | ModerationService 集成到所有 Handler | ✅ | `src/api/listings.rs`, `user_chat.rs`, `user.rs` |
| 4 | `moderation_jobs` 表 + 各表 `moderation_status` 列 | ✅ | `migrations/0011_moderation_jobs.sql` |
| 5 | 接入真实阿里云 IMAN API | ⬜ 待实现 | `src/services/moderation_worker.rs` |
| 6 | 头像待审核占位符 UX（审核通过前显示默认头像） | ⬜ 待实现 | `mobile/` |

**依赖关系：** Item 5 依赖 Item 1-4；Item 6 依赖 Item 5。

### Phase 1 — Quick Wins (XS–S effort)
| # | Item | Severity | Effort | Files |
|---|------|----------|--------|-------|
| 1 | Fix `ProductService::mark_as_sold()` misleading comment | LOW | XS | `src/services/product.rs:17` |
| 2 | Fix `update_order_status` DISABLED comment (endpoint IS wired) | LOW | XS | `src/api/admin.rs:425` |
| 3 | Verify Listing model has image URL field | LOW | XS | `mobile/lib/models/models.dart` |

### Phase 2 — Observability (S–M effort)
| # | Item | Severity | Effort | Files |
|---|------|----------|--------|-------|
| 4 | Enable MetricsService order lifecycle event recorders | MEDIUM | M | `src/api/metrics.rs`, `src/services/mod.rs` |
| 5 | Implement admin middleware for route-level auth | LOW | M | `src/middleware/admin.rs`, `src/api/mod.rs` |

### Phase 3 — Image Loading (S–M effort)
| # | Item | Severity | Effort | Files |
|---|------|----------|--------|-------|
| 6 | Add real thumbnail images to ListingCard and RecommendationCarousel | MEDIUM | M | `mobile/lib/components/price_tag.dart`, `mobile/lib/components/recommendation_carousel.dart` |

### Phase 4 — Future Extensions (M–L effort)
| # | Item | Severity | Effort | Files |
|---|------|----------|--------|-------|
| 7 | Implement `SettlementService` payment integration (Alipay/WeChat Pay/Stripe) | MEDIUM | L | `src/services/settlement.rs` |
| 8 | Implement web secure token storage | LOW | M | `mobile/lib/services/token_storage_*.dart` |

---

## Phase 1 — Quick Wins

### Item 1: Fix `ProductService::mark_as_sold()` Comment

**File**: `src/services/product.rs:17`

**Current state**: Comment says "DISABLED - called from disabled DealReached event". This is misleading because:
- The method IS fully implemented and functional
- It is NOT called from `DealReached` because `OrderService::create_order()` already marks listings as sold atomically in the same transaction
- The method exists for administrative or cleanup use cases

**Fix**: Update the doc comment:
```rust
/// Mark a listing as sold.
///
/// Note: Not called from DealReached because OrderService::create_order()
/// already marks listings as sold atomically. This method exists for
/// administrative use cases (e.g., manual relisting, force-sell).
#[allow(dead_code)]
pub async fn mark_as_sold(&self, listing_id: &str) -> Result<()> {
```

**Effort**: XS — comment-only change

---

### Item 2: Fix `update_order_status` DISABLED Comment

**File**: `src/api/admin.rs:425`

**Current state**: Comment says "(DISABLED)" but the endpoint IS wired and functional at `src/api/mod.rs:264-267`.

**Fix**: Remove "(DISABLED)" from comment:
```rust
/// POST /api/admin/orders/:order_id/status - admin force-sets order status
```

**Effort**: XS — comment-only change

---

### Item 3: Verify Listing Model Has Image URL Field

**File**: `mobile/lib/models/models.dart`

**Current state**: Both `RecommendationCarousel` and `ListingCard` in `PriceTag` show `Icons.inventory_2_outlined` placeholder icons instead of real images. Need to verify the Listing model has an image URL field.

**Action**: Check if `Listing` has `image_urls: Vec<String>` or similar. If not, this becomes a Phase 3 task requiring both backend and frontend changes.

**Effort**: XS — research only

---

## Phase 2 — Observability

### Item 4: Enable MetricsService Order Lifecycle Event Recorders

**Files**:
- `src/api/metrics.rs:157-183`
- `src/services/mod.rs` (event loop)
- `src/services/order.rs` (status transitions)

**Current state**: 5 metric recorder methods are no-ops:
- `record_order_created()` — DISABLED
- `record_order_paid()` — DISABLED
- `record_order_shipped()` — DISABLED
- `record_order_completed()` — DISABLED
- `record_order_cancelled()` — DISABLED

Comments say "orders are disabled" but orders ARE working. The counters are registered in the Prometheus registry; only the recording calls are missing.

**Why it matters**: No observability into order funnel metrics. `/api/metrics` shows zero for all order events.

#### Step 1: Wire into event loop (`src/services/mod.rs`)

```rust
BusinessEvent::DealReached { listing_id, buyer_id, seller_id, final_price } => {
    match order_svc.create_order(&listing_id, &buyer_id, &seller_id, final_price).await {
        Ok(order_id) => {
            state.infra.metrics.record_order_created();
            tracing::info!(order_id, "Order created from DealReached");
        }
        Err(e) => tracing::error!(listing_id, "DealReached order creation failed: {e}"),
    }
}

BusinessEvent::OrderPaid { order_id } => {
    state.infra.metrics.record_order_paid();
    tracing::info!(order_id, "OrderPaid event received");
    // settlement disabled — no further action
}
```

#### Step 2: Wire into OrderService status transitions (`src/services/order.rs`)

In `transition_order_status()`, call the appropriate recorder after each successful DB update:
```rust
match next_status {
    "paid" => { metrics.record_order_paid(); }
    "shipped" => { metrics.record_order_shipped(); }
    "completed" => { metrics.record_order_completed(); }
    "cancelled" => { metrics.record_order_cancelled(); }
    _ => {}
}
```

#### Step 3: Wire admin order cancellation

In `src/api/admin.rs::update_order_status()`, call `record_order_cancelled()` when status transitions to `cancelled`.

**Effort**: M

---

### Item 5: Implement Admin Middleware for Route-Level Auth

**File**: `src/middleware/admin.rs`

**Current state**: `admin_middleware()` at line 12 is a stub that does nothing. Admin auth is handled per-handler via `require_admin()` function in each handler.

**Why it matters**: Per-handler auth is repetitive and error-prone. Middleware is idiomatic Axum and more maintainable.

#### Implementation

1. Add `extract_user_id_and_role_from_token_str()` helper in `src/api/auth.rs`
2. Implement `admin_middleware()`:
```rust
pub async fn admin_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = request
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    let token = match token {
        Some(t) => t,
        None => return ApiError::Unauthorized.into_response(),
    };

    let claims = match verify_jwt(token, &state.secrets.jwt_secret) {
        Ok(c) => c,
        Err(_) => return ApiError::Unauthorized.into_response(),
    };

    if claims.role != "admin" {
        return ApiError::Forbidden.into_response();
    }

    request.extensions_mut().insert(claims.user_id);
    next.run(request).await
}
```

3. Apply to admin routes in `src/api/mod.rs`:
```rust
.route("/api/admin/stats", get(admin::get_admin_stats))
.layer(middleware::from_fn_with_state(state.clone(), admin_middleware))
```

**Note**: Migrate all admin handlers gradually — keep `require_admin()` as fallback during transition.

**Effort**: M

---

## Phase 3 — Image Loading

### Item 6: Real Thumbnail Images

**Files**:
- `mobile/lib/components/price_tag.dart:116` (ListingCard placeholder)
- `mobile/lib/components/recommendation_carousel.dart:99` (_RecommendationCard placeholder)
- `mobile/lib/models/models.dart` (Listing model)
- `src/api/listings.rs` (listing detail response)

**Current state**: Both cards show `Icons.inventory_2_outlined` as placeholder. No real image loading.

#### Step 1: Backend — Add image URL to listing response

Check if `GET /api/listings/:id` returns image URLs. If not, add `thumbnail_url: Option<String>` and `image_urls: Vec<String>` fields to the listing detail response.

#### Step 2: Flutter — Create `ListingImage` component

```dart
/// Widget that shows a listing image with placeholder on null/load failure.
class ListingImage extends StatelessWidget {
  final String? imageUrl;
  final double? width;
  final double? height;
  final BoxFit fit;

  const ListingImage({
    super.key,
    this.imageUrl,
    this.width,
    this.height,
    this.fit = BoxFit.cover,
  });

  @override
  Widget build(BuildContext context) {
    if (imageUrl == null || imageUrl!.isEmpty) {
      return _PlaceholderImage(width: width, height: height);
    }
    return Image.network(
      imageUrl!,
      width: width,
      height: height,
      fit: fit,
      loadingBuilder: (_, child, loadingProgress) {
        if (loadingProgress == null) return child;
        return _PlaceholderImage(width: width, height: height);
      },
      errorBuilder: (_, __, ___) => _PlaceholderImage(width: width, height: height),
    );
  }
}

class _PlaceholderImage extends StatelessWidget {
  final double? width;
  final double? height;
  const _PlaceholderImage({this.width, this.height});
  @override
  Widget build(BuildContext context) {
    return Container(
      width: width,
      height: height,
      color: AppTheme.primary.withValues(alpha: 0.08),
      child: Center(
        child: Icon(Icons.inventory_2_outlined, size: 40, color: AppTheme.primary.withValues(alpha: 0.4)),
      ),
    );
  }
}
```

#### Step 3: Replace placeholders

Replace icon placeholders in `ListingCard` and `_RecommendationCard` with `ListingImage(imageUrl: listing.thumbnailUrl)`.

**Effort**: M (requires backend + frontend coordination)

---

## Phase 4 — Future Extensions

### Item 7: SettlementService Payment Integration

**File**: `src/services/settlement.rs`

**Current state**: `finalize_payment()` and `verify_order_for_payment()` always return `SettlementError::Disabled`. This is a stub placeholder.

**Why it matters**: No actual payment processing. Users can create orders but cannot pay.

**Dependencies**: Requires Item 4 (order metrics) to be in place first for observability.

#### Interface Design

```rust
pub trait PaymentProvider: Send + Sync {
    async fn create_payment(&self, order_id: &str, amount_cents: i64) -> Result<PaymentSession>;
    async fn verify_payment(&self, order_id: &str) -> Result<PaymentStatus>;
    async fn refund(&self, order_id: &str) -> Result<()>;
}

pub struct PaymentSession {
    pub payment_url: String,
    pub transaction_id: String,
}

#[derive(Debug, Clone)]
pub enum PaymentStatus {
    Pending,
    Completed,
    Failed,
    Refunded,
}
```

#### Implementation Order

1. Define `PaymentProvider` trait with China-relevant providers (Alipay, WeChat Pay)
2. Create `aliyun` module implementing `PaymentProvider` using Alipay API
3. Add webhook handler `POST /api/webhooks/payment/:provider`
4. Replace no-op methods with real calls
5. Add `verify_order_for_payment` pre-flight check to order creation

**Effort**: L

---

### Item 8: Web Secure Token Storage

**Files**:
- `mobile/lib/services/token_storage_secure_stub.dart`
- `mobile/lib/services/token_storage.dart`

**Current state**: Web stub returns `null` for reads and does nothing for writes. Web users lose login on page refresh.

#### Implementation

Implement `token_storage_secure_web.dart` using `dart:html` localStorage with basic encryption:

```dart
Future<String?> secureRead(String key) async {
  final storage = html.window.localStorage;
  final encrypted = storage.getItem(key);
  if (encrypted == null) return null;
  return _decrypt(encrypted); // or base64 as basic obfuscation
}

Future<void> secureWrite(String key, String value) async {
  final storage = html.window.localStorage;
  storage.setItem(key, _encrypt(value));
}
```

Update `token_storage.dart` conditional import to include web:
```dart
import 'token_storage_secure_stub.dart'
    if (dart.library.io) 'token_storage_secure_io.dart'
    if (dart.library.html) 'token_storage_secure_web.dart';
```

**Note**: For production, use `package:flutter_web_auth` with HttpOnly cookies instead.

**Effort**: M

---

## Dependency Graph

```
Phase 0 (Moderation System)
    ├── Text Moderation (inline, synchronous) → All handlers
    ├── Image Moderation Worker (async, polling)
    └── IMAN API Integration (future) ← Avatar pending-approval UX

Item 4 (Order Metrics)
    └── Enables monitoring for Item 7

Item 7 (Settlement/Payment)
    ├── Requires: Item 4 (metrics)
    └── Enables: Real payment flow

Item 5 (Admin Middleware)
    └── Independent — can implement anytime

Item 6 (Image Loading)
    └── Requires: Backend thumbnail_url field

Item 8 (Web Secure Storage)
    └── Independent — web-only concern
```

---

## Summary Table

| Item | Status | Severity | Effort | Blocking |
|------|--------|----------|--------|----------|
| **Phase 0: Content Moderation** | | | | |
| M1. Text moderation (keyword/contact/URL → 422) | ✅ Done | — | — | — |
| M2. Image moderation worker (async polling) | ✅ Done | — | — | — |
| M3. ModerationService integrated in all handlers | ✅ Done | — | — | — |
| M4. moderation_jobs table + status columns | ✅ Done | — | — | — |
| M5. Alibaba IMAN API integration | ⬜ Future | — | — | M1-M4 |
| M6. Avatar pending-approval UX | ⬜ Future | — | — | M5 |
| **Phase 1** | | | | |
| 1. Fix `mark_as_sold` comment | Pending | LOW | XS | None |
| 2. Fix `update_order_status` comment | Pending | LOW | XS | None |
| 3. Verify image URL field | Pending | LOW | XS | Item 6 |
| 4. Enable order metrics | Pending | MEDIUM | M | Item 7 |
| 5. Admin middleware | Pending | LOW | M | None |
| 6. Real thumbnails | Pending | MEDIUM | M | Backend change |
| **Phase 4** | | | | |
| 7. Payment integration | Future | MEDIUM | L | Item 4 |
| 8. Web secure storage | Future | LOW | M | None |
