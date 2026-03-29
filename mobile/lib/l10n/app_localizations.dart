import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_zh.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations? of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations);
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('zh'),
  ];

  /// No description provided for @aiAssistantTab.
  ///
  /// In en, this message translates to:
  /// **'AI Assistant'**
  String get aiAssistantTab;

  /// No description provided for @aiError.
  ///
  /// In en, this message translates to:
  /// **'Sorry, something went wrong. Please try again.'**
  String get aiError;

  /// No description provided for @aiGreeting.
  ///
  /// In en, this message translates to:
  /// **'Hello! I\'m your campus secondhand trading assistant. How can I help you today?'**
  String get aiGreeting;

  /// No description provided for @aiWillAutoRecognize.
  ///
  /// In en, this message translates to:
  /// **'AI will auto-recognize item info'**
  String get aiWillAutoRecognize;

  /// No description provided for @allCategories.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get allCategories;

  /// No description provided for @appTitle.
  ///
  /// In en, this message translates to:
  /// **'Campus Marketplace'**
  String get appTitle;

  /// No description provided for @brand.
  ///
  /// In en, this message translates to:
  /// **'Brand'**
  String get brand;

  /// No description provided for @brandLabel.
  ///
  /// In en, this message translates to:
  /// **'Brand'**
  String get brandLabel;

  /// No description provided for @books.
  ///
  /// In en, this message translates to:
  /// **'Books'**
  String get books;

  /// No description provided for @buyNow.
  ///
  /// In en, this message translates to:
  /// **'Buy Now'**
  String get buyNow;

  /// No description provided for @buyer.
  ///
  /// In en, this message translates to:
  /// **'Buyer'**
  String get buyer;

  /// No description provided for @cancel.
  ///
  /// In en, this message translates to:
  /// **'Cancel'**
  String get cancel;

  /// No description provided for @category.
  ///
  /// In en, this message translates to:
  /// **'Category'**
  String get category;

  /// No description provided for @categoryLabel.
  ///
  /// In en, this message translates to:
  /// **'Category'**
  String get categoryLabel;

  /// No description provided for @chinese.
  ///
  /// In en, this message translates to:
  /// **'Chinese (Simplified)'**
  String get chinese;

  /// No description provided for @chat.
  ///
  /// In en, this message translates to:
  /// **'Chat'**
  String get chat;

  /// No description provided for @chatWithSelf.
  ///
  /// In en, this message translates to:
  /// **'Cannot chat with yourself'**
  String get chatWithSelf;

  /// No description provided for @clothingShoes.
  ///
  /// In en, this message translates to:
  /// **'Clothing & Shoes'**
  String get clothingShoes;

  /// No description provided for @comingSoon.
  ///
  /// In en, this message translates to:
  /// **'Coming soon...'**
  String get comingSoon;

  /// No description provided for @condition.
  ///
  /// In en, this message translates to:
  /// **'Condition'**
  String get condition;

  /// No description provided for @conditionLabel.
  ///
  /// In en, this message translates to:
  /// **'Condition'**
  String get conditionLabel;

  /// No description provided for @confirm.
  ///
  /// In en, this message translates to:
  /// **'Confirm'**
  String get confirm;

  /// No description provided for @confirmPassword.
  ///
  /// In en, this message translates to:
  /// **'Confirm Password'**
  String get confirmPassword;

  /// No description provided for @connectionFailedRetry.
  ///
  /// In en, this message translates to:
  /// **'Connection failed, please try again later'**
  String get connectionFailedRetry;

  /// No description provided for @connectionRequestSent.
  ///
  /// In en, this message translates to:
  /// **'Connection request sent, waiting for acceptance'**
  String get connectionRequestSent;

  /// No description provided for @contactSeller.
  ///
  /// In en, this message translates to:
  /// **'Contact Seller'**
  String get contactSeller;

  /// No description provided for @counterOfferAmount.
  ///
  /// In en, this message translates to:
  /// **'Counter offer amount'**
  String get counterOfferAmount;

  /// No description provided for @counterOfferBySeller.
  ///
  /// In en, this message translates to:
  /// **'Seller counter-offered ¥{amount}'**
  String counterOfferBySeller(String amount);

  /// No description provided for @createError.
  ///
  /// In en, this message translates to:
  /// **'Failed to create listing'**
  String get createError;

  /// No description provided for @createListing.
  ///
  /// In en, this message translates to:
  /// **'Create Listing'**
  String get createListing;

  /// No description provided for @createSuccess.
  ///
  /// In en, this message translates to:
  /// **'Listing created successfully'**
  String get createSuccess;

  /// No description provided for @dailyGoods.
  ///
  /// In en, this message translates to:
  /// **'Daily Goods'**
  String get dailyGoods;

  /// No description provided for @defects.
  ///
  /// In en, this message translates to:
  /// **'Defects'**
  String get defects;

  /// No description provided for @defectsLabel.
  ///
  /// In en, this message translates to:
  /// **'Defects'**
  String get defectsLabel;

  /// No description provided for @delete.
  ///
  /// In en, this message translates to:
  /// **'Delete'**
  String get delete;

  /// No description provided for @deleteConfirm.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to delete this listing?'**
  String get deleteConfirm;

  /// No description provided for @description.
  ///
  /// In en, this message translates to:
  /// **'Description'**
  String get description;

  /// No description provided for @descriptionLabel.
  ///
  /// In en, this message translates to:
  /// **'Description'**
  String get descriptionLabel;

  /// No description provided for @digitalAccessories.
  ///
  /// In en, this message translates to:
  /// **'Digital Accessories'**
  String get digitalAccessories;

  /// No description provided for @edit.
  ///
  /// In en, this message translates to:
  /// **'Edit'**
  String get edit;

  /// No description provided for @electronics.
  ///
  /// In en, this message translates to:
  /// **'Electronics'**
  String get electronics;

  /// No description provided for @english.
  ///
  /// In en, this message translates to:
  /// **'English'**
  String get english;

  /// No description provided for @enterValidCounterAmount.
  ///
  /// In en, this message translates to:
  /// **'Please enter a valid counter offer amount'**
  String get enterValidCounterAmount;

  /// No description provided for @error.
  ///
  /// In en, this message translates to:
  /// **'Error'**
  String get error;

  /// No description provided for @fromGallery.
  ///
  /// In en, this message translates to:
  /// **'From gallery'**
  String get fromGallery;

  /// No description provided for @homeTab.
  ///
  /// In en, this message translates to:
  /// **'Home'**
  String get homeTab;

  /// No description provided for @language.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get language;

  /// No description provided for @listingDetail.
  ///
  /// In en, this message translates to:
  /// **'Listing Details'**
  String get listingDetail;

  /// No description provided for @loadFailed.
  ///
  /// In en, this message translates to:
  /// **'Load failed: {error}'**
  String loadFailed(String error);

  /// No description provided for @loading.
  ///
  /// In en, this message translates to:
  /// **'Loading...'**
  String get loading;

  /// No description provided for @loadMore.
  ///
  /// In en, this message translates to:
  /// **'Load more'**
  String get loadMore;

  /// No description provided for @login.
  ///
  /// In en, this message translates to:
  /// **'Login'**
  String get login;

  /// No description provided for @loginError.
  ///
  /// In en, this message translates to:
  /// **'Login error'**
  String get loginError;

  /// No description provided for @loginSuccess.
  ///
  /// In en, this message translates to:
  /// **'Login successful'**
  String get loginSuccess;

  /// No description provided for @logout.
  ///
  /// In en, this message translates to:
  /// **'Logout'**
  String get logout;

  /// No description provided for @logoutConfirm.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to logout?'**
  String get logoutConfirm;

  /// No description provided for @logoutSuccess.
  ///
  /// In en, this message translates to:
  /// **'Logout successful'**
  String get logoutSuccess;

  /// No description provided for @memberSince.
  ///
  /// In en, this message translates to:
  /// **'Member since {date}'**
  String memberSince(String date);

  /// No description provided for @messagesTab.
  ///
  /// In en, this message translates to:
  /// **'Messages'**
  String get messagesTab;

  /// No description provided for @notificationsCenter.
  ///
  /// In en, this message translates to:
  /// **'Notifications'**
  String get notificationsCenter;

  /// No description provided for @notificationsCenterSubtitle.
  ///
  /// In en, this message translates to:
  /// **'System messages and reminders'**
  String get notificationsCenterSubtitle;

  /// No description provided for @myFavorites.
  ///
  /// In en, this message translates to:
  /// **'My Favorites'**
  String get myFavorites;

  /// No description provided for @myFavoritesSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Your favorite items'**
  String get myFavoritesSubtitle;

  /// No description provided for @watchlistEmpty.
  ///
  /// In en, this message translates to:
  /// **'No favorites yet'**
  String get watchlistEmpty;

  /// No description provided for @notificationsEmpty.
  ///
  /// In en, this message translates to:
  /// **'No notifications for now'**
  String get notificationsEmpty;

  /// No description provided for @markAllRead.
  ///
  /// In en, this message translates to:
  /// **'Mark all as read'**
  String get markAllRead;

  /// No description provided for @markAllReadSuccess.
  ///
  /// In en, this message translates to:
  /// **'All notifications marked as read'**
  String get markAllReadSuccess;

  /// No description provided for @myListings.
  ///
  /// In en, this message translates to:
  /// **'My Listings'**
  String get myListings;

  /// No description provided for @myListingsMenu.
  ///
  /// In en, this message translates to:
  /// **'View and manage your listings'**
  String get myListingsMenu;

  /// No description provided for @myListingsTab.
  ///
  /// In en, this message translates to:
  /// **'My Listings'**
  String get myListingsTab;

  /// No description provided for @myOrders.
  ///
  /// In en, this message translates to:
  /// **'My Orders'**
  String get myOrders;

  /// No description provided for @myOrdersSubtitle.
  ///
  /// In en, this message translates to:
  /// **'View purchase history'**
  String get myOrdersSubtitle;

  /// No description provided for @allOrders.
  ///
  /// In en, this message translates to:
  /// **'All'**
  String get allOrders;

  /// No description provided for @buyerOrders.
  ///
  /// In en, this message translates to:
  /// **'As Buyer'**
  String get buyerOrders;

  /// No description provided for @sellerOrders.
  ///
  /// In en, this message translates to:
  /// **'As Seller'**
  String get sellerOrders;

  /// No description provided for @orderAsBuyer.
  ///
  /// In en, this message translates to:
  /// **'As Buyer'**
  String get orderAsBuyer;

  /// No description provided for @orderAsSeller.
  ///
  /// In en, this message translates to:
  /// **'As Seller'**
  String get orderAsSeller;

  /// No description provided for @pay.
  ///
  /// In en, this message translates to:
  /// **'Pay'**
  String get pay;

  /// No description provided for @markPaid.
  ///
  /// In en, this message translates to:
  /// **'Paid'**
  String get markPaid;

  /// No description provided for @reason.
  ///
  /// In en, this message translates to:
  /// **'Reason (optional)'**
  String get reason;

  /// No description provided for @negotiationDetails.
  ///
  /// In en, this message translates to:
  /// **'Negotiation details'**
  String get negotiationDetails;

  /// No description provided for @negotiationExpired.
  ///
  /// In en, this message translates to:
  /// **'Negotiation expired and cancelled'**
  String get negotiationExpired;

  /// No description provided for @connectionAccepted.
  ///
  /// In en, this message translates to:
  /// **'Connection accepted'**
  String get connectionAccepted;

  /// No description provided for @connectionRejected.
  ///
  /// In en, this message translates to:
  /// **'Connection rejected'**
  String get connectionRejected;

  /// No description provided for @negotiationRejected.
  ///
  /// In en, this message translates to:
  /// **'Negotiation rejected'**
  String get negotiationRejected;

  /// No description provided for @noProducts.
  ///
  /// In en, this message translates to:
  /// **'No products available'**
  String get noProducts;

  /// No description provided for @notFound.
  ///
  /// In en, this message translates to:
  /// **'Not found'**
  String get notFound;

  /// No description provided for @operationFailed.
  ///
  /// In en, this message translates to:
  /// **'Operation failed: {error}'**
  String operationFailed(String error);

  /// No description provided for @other.
  ///
  /// In en, this message translates to:
  /// **'Other'**
  String get other;

  /// No description provided for @owner.
  ///
  /// In en, this message translates to:
  /// **'Owner'**
  String get owner;

  /// No description provided for @pendingNegotiation.
  ///
  /// In en, this message translates to:
  /// **'Pending negotiation'**
  String get pendingNegotiation;

  /// No description provided for @password.
  ///
  /// In en, this message translates to:
  /// **'Password'**
  String get password;

  /// No description provided for @price.
  ///
  /// In en, this message translates to:
  /// **'Price'**
  String get price;

  /// No description provided for @priceLabel.
  ///
  /// In en, this message translates to:
  /// **'Price'**
  String get priceLabel;

  /// No description provided for @profile.
  ///
  /// In en, this message translates to:
  /// **'Profile'**
  String get profile;

  /// No description provided for @profileLoadFailed.
  ///
  /// In en, this message translates to:
  /// **'Failed to load profile'**
  String get profileLoadFailed;

  /// No description provided for @profileTab.
  ///
  /// In en, this message translates to:
  /// **'Profile'**
  String get profileTab;

  /// No description provided for @publishTab.
  ///
  /// In en, this message translates to:
  /// **'Publish'**
  String get publishTab;

  /// No description provided for @purchaseFailed.
  ///
  /// In en, this message translates to:
  /// **'Purchase failed, please try again'**
  String get purchaseFailed;

  /// No description provided for @purchaseSuccess.
  ///
  /// In en, this message translates to:
  /// **'Purchase successful! Order created'**
  String get purchaseSuccess;

  /// No description provided for @recognitionFailed.
  ///
  /// In en, this message translates to:
  /// **'Recognition failed: {error}'**
  String recognitionFailed(String error);

  /// No description provided for @recognitionSuccess.
  ///
  /// In en, this message translates to:
  /// **'Recognition successful, info auto-filled'**
  String get recognitionSuccess;

  /// No description provided for @register.
  ///
  /// In en, this message translates to:
  /// **'Register'**
  String get register;

  /// No description provided for @registerError.
  ///
  /// In en, this message translates to:
  /// **'Registration error'**
  String get registerError;

  /// No description provided for @registerSuccess.
  ///
  /// In en, this message translates to:
  /// **'Registration successful'**
  String get registerSuccess;

  /// No description provided for @requestFailed.
  ///
  /// In en, this message translates to:
  /// **'Request failed: {code}'**
  String requestFailed(int code);

  /// No description provided for @retry.
  ///
  /// In en, this message translates to:
  /// **'Retry'**
  String get retry;

  /// No description provided for @searchHint.
  ///
  /// In en, this message translates to:
  /// **'Search products...'**
  String get searchHint;

  /// No description provided for @sellerAcceptedDealComplete.
  ///
  /// In en, this message translates to:
  /// **'Seller accepted, deal complete'**
  String get sellerAcceptedDealComplete;

  /// No description provided for @sellerCounterOffered.
  ///
  /// In en, this message translates to:
  /// **'Seller counter-offered'**
  String get sellerCounterOffered;

  /// No description provided for @send.
  ///
  /// In en, this message translates to:
  /// **'Send'**
  String get send;

  /// No description provided for @sessionExpired.
  ///
  /// In en, this message translates to:
  /// **'Session expired. Please login again.'**
  String get sessionExpired;

  /// No description provided for @settings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get settings;

  /// No description provided for @settingsSubtitle.
  ///
  /// In en, this message translates to:
  /// **'App settings'**
  String get settingsSubtitle;

  /// No description provided for @nickname.
  ///
  /// In en, this message translates to:
  /// **'Nickname'**
  String get nickname;

  /// No description provided for @nicknameChange.
  ///
  /// In en, this message translates to:
  /// **'Change nickname'**
  String get nicknameChange;

  /// No description provided for @nicknameChangeSuccess.
  ///
  /// In en, this message translates to:
  /// **'Nickname updated'**
  String get nicknameChangeSuccess;

  /// No description provided for @nicknameChangeHint.
  ///
  /// In en, this message translates to:
  /// **'Others will see your new nickname after update'**
  String get nicknameChangeHint;

  /// No description provided for @nicknameConflict.
  ///
  /// In en, this message translates to:
  /// **'This nickname is already taken'**
  String get nicknameConflict;

  /// No description provided for @nicknameEmpty.
  ///
  /// In en, this message translates to:
  /// **'Nickname cannot be empty'**
  String get nicknameEmpty;

  /// No description provided for @userAgreement.
  ///
  /// In en, this message translates to:
  /// **'User Agreement'**
  String get userAgreement;

  /// No description provided for @userAgreementTitle.
  ///
  /// In en, this message translates to:
  /// **'User Agreement & Terms'**
  String get userAgreementTitle;

  /// No description provided for @userAgreementSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Understand platform rules and usage responsibilities.'**
  String get userAgreementSubtitle;

  /// No description provided for @sold.
  ///
  /// In en, this message translates to:
  /// **'Sold'**
  String get sold;

  /// No description provided for @status.
  ///
  /// In en, this message translates to:
  /// **'Status'**
  String get status;

  /// No description provided for @submit.
  ///
  /// In en, this message translates to:
  /// **'Submit'**
  String get submit;

  /// No description provided for @takePhoto.
  ///
  /// In en, this message translates to:
  /// **'Take photo'**
  String get takePhoto;

  /// No description provided for @tapCameraIconHint.
  ///
  /// In en, this message translates to:
  /// **'Tap camera icon to take photo or select image'**
  String get tapCameraIconHint;

  /// No description provided for @title.
  ///
  /// In en, this message translates to:
  /// **'Title'**
  String get title;

  /// No description provided for @titleRequired.
  ///
  /// In en, this message translates to:
  /// **'Title is required'**
  String get titleRequired;

  /// No description provided for @totalListings.
  ///
  /// In en, this message translates to:
  /// **'{count} listings'**
  String totalListings(int count);

  /// No description provided for @tradeProtection.
  ///
  /// In en, this message translates to:
  /// **'Trade protection'**
  String get tradeProtection;

  /// No description provided for @tradeProtectionSubtitle.
  ///
  /// In en, this message translates to:
  /// **'Platform escrow + 7-day delivery confirmation'**
  String get tradeProtectionSubtitle;

  /// No description provided for @typeMessage.
  ///
  /// In en, this message translates to:
  /// **'Type a message...'**
  String get typeMessage;

  /// No description provided for @uploadFromCamera.
  ///
  /// In en, this message translates to:
  /// **'Upload from camera'**
  String get uploadFromCamera;

  /// No description provided for @uploadFromGallery.
  ///
  /// In en, this message translates to:
  /// **'Upload from gallery'**
  String get uploadFromGallery;

  /// No description provided for @username.
  ///
  /// In en, this message translates to:
  /// **'Username'**
  String get username;

  /// No description provided for @adminConsole.
  ///
  /// In en, this message translates to:
  /// **'Admin Console'**
  String get adminConsole;

  /// No description provided for @adminConsoleSubtitle.
  ///
  /// In en, this message translates to:
  /// **'System overview & management'**
  String get adminConsoleSubtitle;

  /// No description provided for @adminOnly.
  ///
  /// In en, this message translates to:
  /// **'Admin only'**
  String get adminOnly;

  /// No description provided for @adminStatsTab.
  ///
  /// In en, this message translates to:
  /// **'Stats'**
  String get adminStatsTab;

  /// No description provided for @adminListingsTab.
  ///
  /// In en, this message translates to:
  /// **'Listings'**
  String get adminListingsTab;

  /// No description provided for @adminOrdersTab.
  ///
  /// In en, this message translates to:
  /// **'Orders'**
  String get adminOrdersTab;

  /// No description provided for @adminUsersTab.
  ///
  /// In en, this message translates to:
  /// **'Users'**
  String get adminUsersTab;

  /// No description provided for @adminTotalListings.
  ///
  /// In en, this message translates to:
  /// **'Total Listings'**
  String get adminTotalListings;

  /// No description provided for @adminActive.
  ///
  /// In en, this message translates to:
  /// **'Active'**
  String get adminActive;

  /// No description provided for @adminUsers.
  ///
  /// In en, this message translates to:
  /// **'Users'**
  String get adminUsers;

  /// No description provided for @adminOrders.
  ///
  /// In en, this message translates to:
  /// **'Orders'**
  String get adminOrders;

  /// No description provided for @adminTrend7Days.
  ///
  /// In en, this message translates to:
  /// **'Trend (7 days)'**
  String get adminTrend7Days;

  /// No description provided for @changeRole.
  ///
  /// In en, this message translates to:
  /// **'Change Role'**
  String get changeRole;

  /// No description provided for @markShipped.
  ///
  /// In en, this message translates to:
  /// **'Mark Shipped'**
  String get markShipped;

  /// No description provided for @markCompleted.
  ///
  /// In en, this message translates to:
  /// **'Mark Completed'**
  String get markCompleted;

  /// No description provided for @orderStatusUpdated.
  ///
  /// In en, this message translates to:
  /// **'Order status updated'**
  String get orderStatusUpdated;

  /// No description provided for @userRoleUpdated.
  ///
  /// In en, this message translates to:
  /// **'User role updated'**
  String get userRoleUpdated;

  /// No description provided for @adminTakedown.
  ///
  /// In en, this message translates to:
  /// **'Takedown'**
  String get adminTakedown;

  /// No description provided for @adminTakedownConfirm.
  ///
  /// In en, this message translates to:
  /// **'Confirm Takedown'**
  String get adminTakedownConfirm;

  /// No description provided for @adminTakedownConfirmMessage.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to takedown \"{title}\"?'**
  String adminTakedownConfirmMessage(String title);

  /// No description provided for @adminTakedownSuccess.
  ///
  /// In en, this message translates to:
  /// **'Listing taken down'**
  String get adminTakedownSuccess;

  /// No description provided for @adminBan.
  ///
  /// In en, this message translates to:
  /// **'Ban'**
  String get adminBan;

  /// No description provided for @adminBanConfirm.
  ///
  /// In en, this message translates to:
  /// **'Confirm Ban'**
  String get adminBanConfirm;

  /// No description provided for @adminBanConfirmMessage.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to ban this user? All their sessions will be terminated.'**
  String get adminBanConfirmMessage;

  /// No description provided for @adminBanSuccess.
  ///
  /// In en, this message translates to:
  /// **'User banned'**
  String get adminBanSuccess;

  /// No description provided for @adminUnban.
  ///
  /// In en, this message translates to:
  /// **'Unban'**
  String get adminUnban;

  /// No description provided for @adminUnbanSuccess.
  ///
  /// In en, this message translates to:
  /// **'User unbanned'**
  String get adminUnbanSuccess;

  /// No description provided for @adminSearchListingsPlaceholder.
  ///
  /// In en, this message translates to:
  /// **'Search listings...'**
  String get adminSearchListingsPlaceholder;

  /// No description provided for @adminSearchUsersPlaceholder.
  ///
  /// In en, this message translates to:
  /// **'Search users...'**
  String get adminSearchUsersPlaceholder;

  /// No description provided for @adminNoUsersFound.
  ///
  /// In en, this message translates to:
  /// **'No users found'**
  String get adminNoUsersFound;

  /// No description provided for @adminNoListingsFound.
  ///
  /// In en, this message translates to:
  /// **'No listings found'**
  String get adminNoListingsFound;

  /// No description provided for @adminLoginAs.
  ///
  /// In en, this message translates to:
  /// **'Login as user'**
  String get adminLoginAs;

  /// No description provided for @adminLoginAsSuccess.
  ///
  /// In en, this message translates to:
  /// **'Logged in as {username}'**
  String adminLoginAsSuccess(String username);

  /// No description provided for @adminLoginAsFailed.
  ///
  /// In en, this message translates to:
  /// **'Login failed'**
  String get adminLoginAsFailed;

  /// No description provided for @adminLoginAsConfirm.
  ///
  /// In en, this message translates to:
  /// **'Confirm'**
  String get adminLoginAsConfirm;

  /// No description provided for @adminLoginAsWarning.
  ///
  /// In en, this message translates to:
  /// **'You are about to switch to this user\'s identity'**
  String get adminLoginAsWarning;

  /// No description provided for @adminViewListings.
  ///
  /// In en, this message translates to:
  /// **'View Listings'**
  String get adminViewListings;

  /// No description provided for @orderId.
  ///
  /// In en, this message translates to:
  /// **'Order ID'**
  String get orderId;

  /// No description provided for @orderDetail.
  ///
  /// In en, this message translates to:
  /// **'Order Detail'**
  String get orderDetail;

  /// No description provided for @noOrders.
  ///
  /// In en, this message translates to:
  /// **'No orders'**
  String get noOrders;

  /// No description provided for @conditionLikeNew.
  ///
  /// In en, this message translates to:
  /// **'Like New'**
  String get conditionLikeNew;

  /// No description provided for @conditionGood.
  ///
  /// In en, this message translates to:
  /// **'Good'**
  String get conditionGood;

  /// No description provided for @conditionFair.
  ///
  /// In en, this message translates to:
  /// **'Fair'**
  String get conditionFair;

  /// No description provided for @conditionPoor.
  ///
  /// In en, this message translates to:
  /// **'Poor'**
  String get conditionPoor;

  /// No description provided for @buyerInitiatedNegotiation.
  ///
  /// In en, this message translates to:
  /// **'Buyer initiated negotiation'**
  String get buyerInitiatedNegotiation;

  /// No description provided for @cannotContactSeller.
  ///
  /// In en, this message translates to:
  /// **'Unable to contact seller: missing seller info'**
  String get cannotContactSeller;

  /// No description provided for @itemAlreadyPurchased.
  ///
  /// In en, this message translates to:
  /// **'Oops, this item is too popular, someone beat you to it!'**
  String get itemAlreadyPurchased;

  /// No description provided for @unknown.
  ///
  /// In en, this message translates to:
  /// **'Unknown'**
  String get unknown;

  /// No description provided for @idLabel.
  ///
  /// In en, this message translates to:
  /// **'ID:'**
  String get idLabel;

  /// No description provided for @ownerIdLabel.
  ///
  /// In en, this message translates to:
  /// **'Owner ID:'**
  String get ownerIdLabel;

  /// No description provided for @orderNumber.
  ///
  /// In en, this message translates to:
  /// **'Order #{id}'**
  String orderNumber(String id);

  /// No description provided for @joinedLabel.
  ///
  /// In en, this message translates to:
  /// **'Joined:'**
  String get joinedLabel;

  /// No description provided for @roleLabel.
  ///
  /// In en, this message translates to:
  /// **'Role:'**
  String get roleLabel;

  /// No description provided for @unbanConfirmMessage.
  ///
  /// In en, this message translates to:
  /// **'Are you sure you want to unban user \"{username}\"?'**
  String unbanConfirmMessage(String username);

  /// No description provided for @adminLoginAsAuditLogWarning.
  ///
  /// In en, this message translates to:
  /// **'This operation will log in as the selected user and leave an audit log. Continue?'**
  String get adminLoginAsAuditLogWarning;

  /// No description provided for @impersonationFailed.
  ///
  /// In en, this message translates to:
  /// **'Impersonation failed: {error}'**
  String impersonationFailed(String error);

  /// No description provided for @infoDisclaimer.
  ///
  /// In en, this message translates to:
  /// **'This product is for information publishing only, with no guarantee, no fund intermediary, and no transaction fees.'**
  String get infoDisclaimer;

  /// No description provided for @aboutPlatform.
  ///
  /// In en, this message translates to:
  /// **'About This Platform'**
  String get aboutPlatform;

  /// No description provided for @aboutPlatformSubtitle.
  ///
  /// In en, this message translates to:
  /// **'How this platform works and key safety notice.'**
  String get aboutPlatformSubtitle;

  /// No description provided for @infoPublishing.
  ///
  /// In en, this message translates to:
  /// **'Information Publishing'**
  String get infoPublishing;

  /// No description provided for @infoPublishingDesc.
  ///
  /// In en, this message translates to:
  /// **'This platform is for information publishing only. Users share listing information through posts. No transactions or payments occur on this platform.'**
  String get infoPublishingDesc;

  /// No description provided for @contactThroughChat.
  ///
  /// In en, this message translates to:
  /// **'Contact Through Chat'**
  String get contactThroughChat;

  /// No description provided for @contactThroughChatDesc.
  ///
  /// In en, this message translates to:
  /// **'Contact sellers directly through the in-app chat feature. Communicate details and arrange transactions offline.'**
  String get contactThroughChatDesc;

  /// No description provided for @safetyTips.
  ///
  /// In en, this message translates to:
  /// **'Safety Tips'**
  String get safetyTips;

  /// No description provided for @safetyTipsDesc.
  ///
  /// In en, this message translates to:
  /// **'Meet in safe public places when exchanging items. Verify item condition before completing any offline arrangement.'**
  String get safetyTipsDesc;

  /// No description provided for @platformDisclaimer.
  ///
  /// In en, this message translates to:
  /// **'This platform serves as an information listing service only. Any offline transactions are at your own risk. Please stay vigilant and protect your personal safety and property.'**
  String get platformDisclaimer;

  /// No description provided for @recommendedForYou.
  ///
  /// In en, this message translates to:
  /// **'For You'**
  String get recommendedForYou;

  /// No description provided for @similarRecommendations.
  ///
  /// In en, this message translates to:
  /// **'Similar Items'**
  String get similarRecommendations;

  /// No description provided for @camera.
  ///
  /// In en, this message translates to:
  /// **'Camera'**
  String get camera;

  /// No description provided for @gallery.
  ///
  /// In en, this message translates to:
  /// **'Gallery'**
  String get gallery;

  /// No description provided for @uploading.
  ///
  /// In en, this message translates to:
  /// **'Uploading'**
  String get uploading;

  /// No description provided for @avatarUpdated.
  ///
  /// In en, this message translates to:
  /// **'Avatar updated'**
  String get avatarUpdated;

  /// No description provided for @uploadFailed.
  ///
  /// In en, this message translates to:
  /// **'Upload failed'**
  String get uploadFailed;

  /// No description provided for @emailLabel.
  ///
  /// In en, this message translates to:
  /// **'Email'**
  String get emailLabel;

  /// No description provided for @emailChange.
  ///
  /// In en, this message translates to:
  /// **'Change Email'**
  String get emailChange;

  /// No description provided for @emailChangeHint.
  ///
  /// In en, this message translates to:
  /// **'Enter @email.ncu.edu.cn email'**
  String get emailChangeHint;

  /// No description provided for @emailDomainError.
  ///
  /// In en, this message translates to:
  /// **'Must use @email.ncu.edu.cn email'**
  String get emailDomainError;

  /// No description provided for @emailChangeSuccess.
  ///
  /// In en, this message translates to:
  /// **'Email updated'**
  String get emailChangeSuccess;

  /// No description provided for @notSet.
  ///
  /// In en, this message translates to:
  /// **'Not set'**
  String get notSet;
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'zh'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'zh':
      return AppLocalizationsZh();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
