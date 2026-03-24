import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'l10n/app_localizations.dart';
import 'router/app_router.dart';
import 'theme/app_theme.dart';
import 'services/locale_service.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await LocaleService.init();
  runApp(const Good4NCUApp());
}

class Good4NCUApp extends StatefulWidget {
  const Good4NCUApp({super.key});

  @override
  State<Good4NCUApp> createState() => _Good4NCUAppState();
}

class _Good4NCUAppState extends State<Good4NCUApp> {
  final LocaleNotifier _localeNotifier = LocaleNotifier();

  @override
  void dispose() {
    _localeNotifier.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return LocaleProvider(
      notifier: _localeNotifier,
      child: ListenableBuilder(
        listenable: _localeNotifier,
        builder: (context, _) {
          return MaterialApp.router(
            title: 'Good4NCU',
            theme: AppTheme.light,
            darkTheme: AppTheme.dark,
            themeMode: ThemeMode.system,
            locale: _localeNotifier.locale,
            localizationsDelegates: const [
              AppLocalizations.delegate,
              GlobalMaterialLocalizations.delegate,
              GlobalWidgetsLocalizations.delegate,
              GlobalCupertinoLocalizations.delegate,
            ],
            supportedLocales: const [
              Locale('en'),
              Locale('zh'),
            ],
            routerConfig: appRouter,
            debugShowCheckedModeBanner: false,
          );
        },
      ),
    );
  }
}
