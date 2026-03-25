// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'Campus Marketplace';

  @override
  String get homeTab => 'Home';

  @override
  String get aiAssistantTab => 'AI Assistant';

  @override
  String get messagesTab => 'Messages';

  @override
  String get publishTab => 'Publish';

  @override
  String get myListingsTab => 'My Listings';

  @override
  String get profileTab => 'Profile';

  @override
  String get searchHint => 'Search products...';

  @override
  String get allCategories => 'All';

  @override
  String get electronics => 'Electronics';

  @override
  String get books => 'Books';

  @override
  String get digitalAccessories => 'Digital Accessories';

  @override
  String get dailyGoods => 'Daily Goods';

  @override
  String get clothingShoes => 'Clothing & Shoes';

  @override
  String get other => 'Other';

  @override
  String get noProducts => 'No products available';

  @override
  String get retry => 'Retry';

  @override
  String get login => 'Login';

  @override
  String get register => 'Register';

  @override
  String get username => 'Username';

  @override
  String get password => 'Password';

  @override
  String get confirmPassword => 'Confirm Password';

  @override
  String get loginSuccess => 'Login successful';

  @override
  String get registerSuccess => 'Registration successful';

  @override
  String get loginError => 'Login error';

  @override
  String get registerError => 'Registration error';

  @override
  String get sessionExpired => 'Session expired. Please login again.';

  @override
  String get logout => 'Logout';

  @override
  String get logoutSuccess => 'Logout successful';

  @override
  String get createListing => 'Create Listing';

  @override
  String get title => 'Title';

  @override
  String get category => 'Category';

  @override
  String get brand => 'Brand';

  @override
  String get condition => 'Condition';

  @override
  String get price => 'Price';

  @override
  String get description => 'Description';

  @override
  String get defects => 'Defects';

  @override
  String get submit => 'Submit';

  @override
  String get cancel => 'Cancel';

  @override
  String get createSuccess => 'Listing created successfully';

  @override
  String get createError => 'Failed to create listing';

  @override
  String get titleRequired => 'Title is required';

  @override
  String get myListings => 'My Listings';

  @override
  String get active => 'Active';

  @override
  String get sold => 'Sold';

  @override
  String get deleted => 'Deleted';

  @override
  String get edit => 'Edit';

  @override
  String get delete => 'Delete';

  @override
  String get deleteConfirm => 'Are you sure you want to delete this listing?';

  @override
  String get profile => 'Profile';

  @override
  String memberSince(String date) {
    return 'Member since $date';
  }

  @override
  String totalListings(int count) {
    return '$count listings';
  }

  @override
  String get chat => 'Chat';

  @override
  String get typeMessage => 'Type a message...';

  @override
  String get send => 'Send';

  @override
  String get aiGreeting =>
      'Hello! I\'m your campus secondhand trading assistant. How can I help you today?';

  @override
  String get aiError => 'Sorry, something went wrong. Please try again.';

  @override
  String get listingDetail => 'Listing Details';

  @override
  String get contactSeller => 'Contact Seller';

  @override
  String get buyNow => 'Buy Now';

  @override
  String get priceLabel => 'Price';

  @override
  String get conditionLabel => 'Condition';

  @override
  String get categoryLabel => 'Category';

  @override
  String get brandLabel => 'Brand';

  @override
  String get defectsLabel => 'Defects';

  @override
  String get descriptionLabel => 'Description';

  @override
  String get owner => 'Owner';

  @override
  String get status => 'Status';

  @override
  String get createdAt => 'Created at';

  @override
  String get notFound => 'Not found';

  @override
  String get error => 'Error';

  @override
  String get loading => 'Loading...';

  @override
  String requestFailed(int code) {
    return 'Request failed: $code';
  }

  @override
  String get language => 'Language';

  @override
  String get logoutConfirm => 'Are you sure you want to logout?';

  @override
  String get myListingsMenu => 'View and manage your listings';

  @override
  String get myOrders => 'My Orders';

  @override
  String get myOrdersSubtitle => 'View purchase history';

  @override
  String get myFavorites => 'My Favorites';

  @override
  String get myFavoritesSubtitle => 'Your favorite items';

  @override
  String get settings => 'Settings';

  @override
  String get settingsSubtitle => 'App settings';

  @override
  String get comingSoon => 'Coming soon...';
}
