// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get aiAssistantTab => 'AI助手';

  @override
  String get aiError => '抱歉，出现了一些问题，请重试。';

  @override
  String get aiGreeting => '你好！我是校园二手交易平台的智能助手。有什么我可以帮你的吗？';

  @override
  String get aiWillAutoRecognize => 'AI将自动识别商品信息';

  @override
  String get allCategories => '全部';

  @override
  String get appTitle => '校园集市';

  @override
  String get brand => '品牌';

  @override
  String get brandLabel => '品牌';

  @override
  String get books => '图书';

  @override
  String get buyNow => '立即购买';

  @override
  String get buyer => '买家';

  @override
  String get cancel => '取消';

  @override
  String get category => '分类';

  @override
  String get categoryLabel => '分类';

  @override
  String get chinese => '简体中文';

  @override
  String get chat => '聊天';

  @override
  String get chatWithSelf => '不能和自己聊天';

  @override
  String get clothingShoes => '服饰鞋包';

  @override
  String get comingSoon => '即将推出...';

  @override
  String get condition => '成色';

  @override
  String get conditionLabel => '成色';

  @override
  String get confirm => '确认';

  @override
  String get confirmPassword => '确认密码';

  @override
  String get connectionFailedRetry => '连接失败，请稍后重试';

  @override
  String get connectionRequestSent => '已发送连接请求，等待对方接受';

  @override
  String get contactSeller => '联系卖家';

  @override
  String get counterOfferAmount => '还价金额';

  @override
  String counterOfferBySeller(String amount) {
    return '卖家还价 ¥$amount';
  }

  @override
  String get createError => '发布失败';

  @override
  String get createListing => '发布商品';

  @override
  String get createSuccess => '商品发布成功';

  @override
  String get dailyGoods => '生活用品';

  @override
  String get defects => '缺陷';

  @override
  String get defectsLabel => '缺陷';

  @override
  String get delete => '删除';

  @override
  String get deleteConfirm => '确定要删除这件商品吗？';

  @override
  String get description => '描述';

  @override
  String get descriptionLabel => '描述';

  @override
  String get digitalAccessories => '数码配件';

  @override
  String get edit => '编辑';

  @override
  String get electronics => '电子产品';

  @override
  String get english => 'English';

  @override
  String get enterValidCounterAmount => '请输入有效的还价金额';

  @override
  String get error => '错误';

  @override
  String get fromGallery => '相册';

  @override
  String get homeTab => '首页';

  @override
  String get language => '语言';

  @override
  String get listingDetail => '商品详情';

  @override
  String loadFailed(String error) {
    return '加载失败: $error';
  }

  @override
  String get loading => '加载中...';

  @override
  String get loadMore => '加载更多';

  @override
  String get login => '登录';

  @override
  String get loginError => '登录错误';

  @override
  String get loginSuccess => '登录成功';

  @override
  String get logout => '退出登录';

  @override
  String get logoutConfirm => '确定要退出登录吗？';

  @override
  String get logoutSuccess => '退出登录成功';

  @override
  String memberSince(String date) {
    return '注册于 $date';
  }

  @override
  String get messagesTab => '消息';

  @override
  String get notificationsCenter => '通知中心';

  @override
  String get notificationsCenterSubtitle => '系统消息与提醒';

  @override
  String get myFavorites => '我的收藏';

  @override
  String get myFavoritesSubtitle => '您收藏的商品';

  @override
  String get watchlistEmpty => '你还没有收藏商品';

  @override
  String get notificationsEmpty => '暂无通知';

  @override
  String get markAllRead => '全部已读';

  @override
  String get markAllReadSuccess => '已将全部通知标记为已读';

  @override
  String get myListings => '我的发布';

  @override
  String get myListingsMenu => '查看和管理您的商品';

  @override
  String get myListingsTab => '我的发布';

  @override
  String get myOrders => '我的订单';

  @override
  String get myOrdersSubtitle => '查看购买记录';

  @override
  String get allOrders => '全部';

  @override
  String get buyerOrders => '作为买家';

  @override
  String get sellerOrders => '作为卖家';

  @override
  String get orderAsBuyer => '作为买家';

  @override
  String get orderAsSeller => '作为卖家';

  @override
  String get pay => '支付';

  @override
  String get markPaid => '已支付';

  @override
  String get reason => '取消原因（选填）';

  @override
  String get negotiationDetails => '议价详情';

  @override
  String get negotiationExpired => '议价已超时取消';

  @override
  String get connectionAccepted => '已接受连接';

  @override
  String get connectionRejected => '已拒绝连接';

  @override
  String get negotiationRejected => '议价已拒绝';

  @override
  String get noProducts => '暂无商品';

  @override
  String get notFound => '未找到';

  @override
  String operationFailed(String error) {
    return '操作失败: $error';
  }

  @override
  String get other => '其他';

  @override
  String get owner => '卖家';

  @override
  String get pendingNegotiation => '待处理议价';

  @override
  String get password => '密码';

  @override
  String get price => '价格';

  @override
  String get priceLabel => '价格';

  @override
  String get profile => '个人信息';

  @override
  String get profileLoadFailed => '个人资料加载失败';

  @override
  String get profileTab => '我的';

  @override
  String get publishTab => '发布';

  @override
  String get purchaseFailed => '购买失败，请稍后重试';

  @override
  String get purchaseSuccess => '购买成功！订单已创建';

  @override
  String recognitionFailed(String error) {
    return '识别失败: $error';
  }

  @override
  String get recognitionSuccess => '识别成功，已自动填充信息';

  @override
  String get register => '注册';

  @override
  String get registerError => '注册错误';

  @override
  String get registerSuccess => '注册成功';

  @override
  String requestFailed(int code) {
    return '请求失败: $code';
  }

  @override
  String get retry => '重试';

  @override
  String get searchHint => '搜索商品...';

  @override
  String get sellerAcceptedDealComplete => '卖家已接受，交易完成';

  @override
  String get sellerCounterOffered => '卖家已还价';

  @override
  String get send => '发送';

  @override
  String get sessionExpired => '会话已过期，请重新登录';

  @override
  String get settings => '设置';

  @override
  String get settingsSubtitle => '应用设置';

  @override
  String get nickname => '昵称';

  @override
  String get nicknameChange => '修改昵称';

  @override
  String get nicknameChangeSuccess => '昵称已更新';

  @override
  String get nicknameChangeHint => '设置后其他人将看到你的新昵称';

  @override
  String get nicknameConflict => '该昵称已被使用';

  @override
  String get nicknameEmpty => '昵称不能为空';

  @override
  String get userAgreement => '用户协议';

  @override
  String get userAgreementTitle => '用户协议与条款';

  @override
  String get userAgreementSubtitle => '了解平台规则与使用责任范围。';

  @override
  String get sold => '已售';

  @override
  String get status => '状态';

  @override
  String get submit => '提交';

  @override
  String get takePhoto => '拍照';

  @override
  String get tapCameraIconHint => '点击右上角相机图标拍照或选择图片';

  @override
  String get title => '标题';

  @override
  String get titleRequired => '请输入标题';

  @override
  String totalListings(int count) {
    return '共 $count 件商品';
  }

  @override
  String get tradeProtection => '交易保障';

  @override
  String get tradeProtectionSubtitle => '平台托管 + 7天确认收货';

  @override
  String get typeMessage => '输入消息...';

  @override
  String get uploadFromCamera => '拍照上传';

  @override
  String get uploadFromGallery => '相册上传';

  @override
  String get username => '用户名';

  @override
  String get adminConsole => '管理后台';

  @override
  String get adminConsoleSubtitle => '系统概览与管理';

  @override
  String get adminOnly => '仅管理员可用';

  @override
  String get adminStatsTab => '统计';

  @override
  String get adminListingsTab => '商品';

  @override
  String get adminOrdersTab => '订单';

  @override
  String get adminUsersTab => '用户';

  @override
  String get adminTotalListings => '商品总数';

  @override
  String get adminActive => '在售';

  @override
  String get adminUsers => '用户总数';

  @override
  String get adminOrders => '订单总数';

  @override
  String get adminTrend7Days => '趋势 (7日)';

  @override
  String get changeRole => '修改角色';

  @override
  String get markShipped => '标记已发货';

  @override
  String get markCompleted => '标记已完成';

  @override
  String get orderStatusUpdated => '订单状态已更新';

  @override
  String get userRoleUpdated => '用户角色已更新';

  @override
  String get adminTakedown => '强制下架 (Takedown)';

  @override
  String get adminTakedownConfirm => '确认下架';

  @override
  String adminTakedownConfirmMessage(String title) {
    return '确定要强制下架 \"$title\" 吗？';
  }

  @override
  String get adminTakedownSuccess => '商品已强制下架';

  @override
  String get adminBan => '封禁用户 (Ban)';

  @override
  String get adminBanConfirm => '确认封禁';

  @override
  String get adminBanConfirmMessage => '确定要封禁该用户吗？封禁后该用户所有登录状态将被清除。';

  @override
  String get adminBanSuccess => '用户已被封禁';

  @override
  String get adminUnban => '解封用户 (Unban)';

  @override
  String get adminUnbanSuccess => '用户已解封';

  @override
  String get adminSearchListingsPlaceholder => '搜索商品...';

  @override
  String get adminSearchUsersPlaceholder => '搜索用户...';

  @override
  String get adminNoUsersFound => '未找到用户';

  @override
  String get adminNoListingsFound => '未找到商品';

  @override
  String get adminLoginAs => '以该用户登录';

  @override
  String adminLoginAsSuccess(String username) {
    return '已以 $username 身份登录';
  }

  @override
  String get adminLoginAsFailed => '登录失败';

  @override
  String get adminLoginAsConfirm => '确认登录';

  @override
  String get adminLoginAsWarning => '即将切换到该用户身份';

  @override
  String get adminViewListings => '查看商品';

  @override
  String get orderId => '订单号';

  @override
  String get orderDetail => '订单详情';

  @override
  String get noOrders => '暂无订单';

  @override
  String get conditionLikeNew => '几乎全新';

  @override
  String get conditionGood => '较好';

  @override
  String get conditionFair => '一般';

  @override
  String get conditionPoor => '较差';

  @override
  String get buyerInitiatedNegotiation => '买家发起议价';

  @override
  String get cannotContactSeller => '无法联系卖家：缺少卖家信息';

  @override
  String get itemAlreadyPurchased => '哎呀，该商品太火爆，已经被别人抢先一步啦！';

  @override
  String get unknown => '未知';

  @override
  String get idLabel => 'ID：';

  @override
  String get ownerIdLabel => '卖家ID：';

  @override
  String orderNumber(String id) {
    return '订单 #$id';
  }

  @override
  String get joinedLabel => '注册时间：';

  @override
  String get roleLabel => '角色：';

  @override
  String unbanConfirmMessage(String username) {
    return '确定要解封用户 \"$username\" 吗？';
  }

  @override
  String get adminLoginAsAuditLogWarning => '此操作将以选定用户的身份登录并留下审计日志，确定吗？';

  @override
  String impersonationFailed(String error) {
    return '身份切换失败：$error';
  }

  @override
  String get infoDisclaimer => '本产品仅做信息发布，无担保和资金中介，也不收手续费';

  @override
  String get aboutPlatform => '关于我们';

  @override
  String get aboutPlatformSubtitle => '了解平台定位、使用方式与安全提醒。';

  @override
  String get infoPublishing => '信息发布';

  @override
  String get infoPublishingDesc => '本平台仅提供信息发布服务，用户通过发帖分享商品信息。平台不参与任何交易或支付环节。';

  @override
  String get contactThroughChat => '通过聊天联系';

  @override
  String get contactThroughChatDesc => '可直接通过应用内聊天功能联系卖家，沟通细节并自行安排线下交易。';

  @override
  String get safetyTips => '安全提示';

  @override
  String get safetyTipsDesc => '交换物品时请选择安全的公共场所，交易前请仔细核实物品状况。';

  @override
  String get platformDisclaimer =>
      '本平台仅作为信息发布服务提供方，任何线下交易行为风险自担。请保持警惕，注意人身和财产安全。';

  @override
  String get recommendedForYou => '为你推荐';

  @override
  String get similarRecommendations => '相似推荐';

  @override
  String get camera => '拍照';

  @override
  String get gallery => '相册';

  @override
  String get uploading => '上传中';

  @override
  String get avatarUpdated => '头像已更新';

  @override
  String get uploadFailed => '上传失败';

  @override
  String get emailLabel => '邮箱';

  @override
  String get emailChange => '修改邮箱';

  @override
  String get emailChangeHint => '输入 @email.ncu.edu.cn 邮箱';

  @override
  String get emailDomainError => '必须使用 @email.ncu.edu.cn 邮箱';

  @override
  String get emailChangeSuccess => '邮箱已更新';

  @override
  String get notSet => '未设置';
}
