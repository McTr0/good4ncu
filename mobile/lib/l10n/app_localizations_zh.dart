// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get appTitle => '校园集市';

  @override
  String get homeTab => '首页';

  @override
  String get aiAssistantTab => 'AI助手';

  @override
  String get messagesTab => '消息';

  @override
  String get publishTab => '发布';

  @override
  String get myListingsTab => '我的发布';

  @override
  String get profileTab => '我的';

  @override
  String get searchHint => '搜索商品...';

  @override
  String get allCategories => '全部';

  @override
  String get electronics => '电子产品';

  @override
  String get books => '书籍教材';

  @override
  String get digitalAccessories => '数码配件';

  @override
  String get dailyGoods => '生活用品';

  @override
  String get clothingShoes => '服饰鞋包';

  @override
  String get other => '其他';

  @override
  String get noProducts => '暂无商品';

  @override
  String get retry => '重试';

  @override
  String get login => '登录';

  @override
  String get register => '注册';

  @override
  String get username => '用户名';

  @override
  String get password => '密码';

  @override
  String get confirmPassword => '确认密码';

  @override
  String get loginSuccess => '登录成功';

  @override
  String get registerSuccess => '注册成功';

  @override
  String get loginError => '登录错误';

  @override
  String get registerError => '注册错误';

  @override
  String get sessionExpired => '会话已过期，请重新登录';

  @override
  String get logout => '退出登录';

  @override
  String get logoutSuccess => '退出登录成功';

  @override
  String get createListing => '发布商品';

  @override
  String get title => '标题';

  @override
  String get category => '分类';

  @override
  String get brand => '品牌';

  @override
  String get condition => '成色';

  @override
  String get price => '价格';

  @override
  String get description => '描述';

  @override
  String get defects => '缺陷';

  @override
  String get submit => '提交';

  @override
  String get cancel => '取消';

  @override
  String get createSuccess => '商品发布成功';

  @override
  String get createError => '发布失败';

  @override
  String get titleRequired => '请输入标题';

  @override
  String get myListings => '我的发布';

  @override
  String get active => '在售';

  @override
  String get sold => '已售';

  @override
  String get deleted => '已下架';

  @override
  String get edit => '编辑';

  @override
  String get delete => '删除';

  @override
  String get deleteConfirm => '确定要删除这件商品吗？';

  @override
  String get profile => '个人信息';

  @override
  String memberSince(String date) {
    return '注册于 $date';
  }

  @override
  String totalListings(int count) {
    return '共 $count 件商品';
  }

  @override
  String get chat => '聊天';

  @override
  String get typeMessage => '输入消息...';

  @override
  String get send => '发送';

  @override
  String get aiGreeting => '你好！我是校园二手交易平台的智能助手。有什么我可以帮你的吗？';

  @override
  String get aiError => '抱歉，出现了一些问题，请重试。';

  @override
  String get listingDetail => '商品详情';

  @override
  String get contactSeller => '联系卖家';

  @override
  String get buyNow => '立即购买';

  @override
  String get priceLabel => '价格';

  @override
  String get conditionLabel => '成色';

  @override
  String get categoryLabel => '分类';

  @override
  String get brandLabel => '品牌';

  @override
  String get defectsLabel => '缺陷';

  @override
  String get descriptionLabel => '描述';

  @override
  String get owner => '卖家';

  @override
  String get status => '状态';

  @override
  String get createdAt => '发布时间';

  @override
  String get notFound => '未找到';

  @override
  String get error => '错误';

  @override
  String get loading => '加载中...';

  @override
  String requestFailed(int code) {
    return '请求失败: $code';
  }

  @override
  String get language => '语言';

  @override
  String get logoutConfirm => '确定要退出登录吗？';

  @override
  String get myListingsMenu => '查看和管理您的商品';

  @override
  String get myOrders => '我的订单';

  @override
  String get myOrdersSubtitle => '查看购买记录';

  @override
  String get myFavorites => '我的收藏';

  @override
  String get myFavoritesSubtitle => '您收藏的商品';

  @override
  String get settings => '设置';

  @override
  String get settingsSubtitle => '应用设置';

  @override
  String get comingSoon => '即将推出...';
}
