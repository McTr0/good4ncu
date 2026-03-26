// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get aiAssistantTab => 'AI Assistant';

  @override
  String get aiError => 'Sorry, something went wrong. Please try again.';

  @override
  String get aiGreeting =>
      'Hello! I\'m your campus secondhand trading assistant. How can I help you today?';

  @override
  String get aiWillAutoRecognize => 'AI will auto-recognize item info';

  @override
  String get allCategories => 'All';

  @override
  String get appTitle => 'Campus Marketplace';

  @override
  String get brand => 'Brand';

  @override
  String get brandLabel => 'Brand';

  @override
  String get books => 'Books';

  @override
  String get buyNow => 'Buy Now';

  @override
  String get cancel => 'Cancel';

  @override
  String get category => 'Category';

  @override
  String get categoryLabel => 'Category';

  @override
  String get chat => 'Chat';

  @override
  String get chatWithSelf => 'Cannot chat with yourself';

  @override
  String get clothingShoes => 'Clothing & Shoes';

  @override
  String get comingSoon => 'Coming soon...';

  @override
  String get condition => 'Condition';

  @override
  String get conditionLabel => 'Condition';

  @override
  String get confirmPassword => 'Confirm Password';

  @override
  String get connectionFailedRetry =>
      'Connection failed, please try again later';

  @override
  String get connectionRequestSent =>
      'Connection request sent, waiting for acceptance';

  @override
  String get contactSeller => 'Contact Seller';

  @override
  String get counterOfferAmount => 'Counter offer amount';

  @override
  String counterOfferBySeller(String amount) {
    return 'Seller counter-offered ¥$amount';
  }

  @override
  String get createError => 'Failed to create listing';

  @override
  String get createListing => 'Create Listing';

  @override
  String get createSuccess => 'Listing created successfully';

  @override
  String get dailyGoods => 'Daily Goods';

  @override
  String get defects => 'Defects';

  @override
  String get defectsLabel => 'Defects';

  @override
  String get delete => 'Delete';

  @override
  String get deleteConfirm => 'Are you sure you want to delete this listing?';

  @override
  String get description => 'Description';

  @override
  String get descriptionLabel => 'Description';

  @override
  String get digitalAccessories => 'Digital Accessories';

  @override
  String get edit => 'Edit';

  @override
  String get electronics => 'Electronics';

  @override
  String get enterValidCounterAmount =>
      'Please enter a valid counter offer amount';

  @override
  String get error => 'Error';

  @override
  String get fromGallery => 'From gallery';

  @override
  String get homeTab => 'Home';

  @override
  String get language => 'Language';

  @override
  String get listingDetail => 'Listing Details';

  @override
  String loadFailed(String error) {
    return 'Load failed: $error';
  }

  @override
  String get loading => 'Loading...';

  @override
  String get login => 'Login';

  @override
  String get loginError => 'Login error';

  @override
  String get loginSuccess => 'Login successful';

  @override
  String get logout => 'Logout';

  @override
  String get logoutConfirm => 'Are you sure you want to logout?';

  @override
  String get logoutSuccess => 'Logout successful';

  @override
  String memberSince(String date) {
    return 'Member since $date';
  }

  @override
  String get messagesTab => 'Messages';

  @override
  String get myFavorites => 'My Favorites';

  @override
  String get myFavoritesSubtitle => 'Your favorite items';

  @override
  String get myListings => 'My Listings';

  @override
  String get myListingsMenu => 'View and manage your listings';

  @override
  String get myListingsTab => 'My Listings';

  @override
  String get myOrders => 'My Orders';

  @override
  String get myOrdersSubtitle => 'View purchase history';

  @override
  String get negotiationDetails => 'Negotiation details';

  @override
  String get negotiationExpired => 'Negotiation expired and cancelled';

  @override
  String get negotiationRejected => 'Negotiation rejected';

  @override
  String get noProducts => 'No products available';

  @override
  String get notFound => 'Not found';

  @override
  String operationFailed(String error) {
    return 'Operation failed: $error';
  }

  @override
  String get other => 'Other';

  @override
  String get owner => 'Owner';

  @override
  String get pendingNegotiation => 'Pending negotiation';

  @override
  String get password => 'Password';

  @override
  String get price => 'Price';

  @override
  String get priceLabel => 'Price';

  @override
  String get profile => 'Profile';

  @override
  String get profileTab => 'Profile';

  @override
  String get publishTab => 'Publish';

  @override
  String get purchaseFailed => 'Purchase failed, please try again';

  @override
  String get purchaseSuccess => 'Purchase successful! Order created';

  @override
  String recognitionFailed(String error) {
    return 'Recognition failed: $error';
  }

  @override
  String get recognitionSuccess => 'Recognition successful, info auto-filled';

  @override
  String get register => 'Register';

  @override
  String get registerError => 'Registration error';

  @override
  String get registerSuccess => 'Registration successful';

  @override
  String requestFailed(int code) {
    return 'Request failed: $code';
  }

  @override
  String get retry => 'Retry';

  @override
  String get searchHint => 'Search products...';

  @override
  String get sellerAcceptedDealComplete => 'Seller accepted, deal complete';

  @override
  String get sellerCounterOffered => 'Seller counter-offered';

  @override
  String get send => 'Send';

  @override
  String get sessionExpired => 'Session expired. Please login again.';

  @override
  String get settings => 'Settings';

  @override
  String get settingsSubtitle => 'App settings';

  @override
  String get sold => 'Sold';

  @override
  String get status => 'Status';

  @override
  String get submit => 'Submit';

  @override
  String get takePhoto => 'Take photo';

  @override
  String get tapCameraIconHint =>
      'Tap camera icon to take photo or select image';

  @override
  String get title => 'Title';

  @override
  String get titleRequired => 'Title is required';

  @override
  String totalListings(int count) {
    return '$count listings';
  }

  @override
  String get tradeProtection => 'Trade protection';

  @override
  String get typeMessage => 'Type a message...';

  @override
  String get uploadFromCamera => 'Upload from camera';

  @override
  String get uploadFromGallery => 'Upload from gallery';

  @override
  String get username => 'Username';
}
