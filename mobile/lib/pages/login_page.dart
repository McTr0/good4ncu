import 'package:flutter/material.dart';
import '../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import 'package:go_router/go_router.dart';
import '../services/api_service.dart';

class LoginPage extends StatefulWidget {
  const LoginPage({super.key});

  @override
  State<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends State<LoginPage> {
  final _usernameController = TextEditingController();
  final _passwordController = TextEditingController();
  final _confirmPasswordController = TextEditingController();
  final ApiService _apiService = ApiService();
  bool _isLogin = true;
  bool _isLoading = false;

  Future<void> _submit() async {
    final l = AppLocalizations.of(context)!;
    setState(() => _isLoading = true);
    try {
      final username = _usernameController.text.trim();
      final password = _passwordController.text;

      if (username.isEmpty || password.isEmpty) {
        throw Exception(l.loginError);
      }

      if (!_isLogin && password != _confirmPasswordController.text) {
        throw Exception(l.registerError);
      }

      String token;
      if (_isLogin) {
        token = await _apiService.login(username, password);
      } else {
        token = await _apiService.register(username, password);
      }

      if (token.isNotEmpty) {
        if (mounted) {
          context.go('/');
        }
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('${l.error}: ${e.toString()}')));
      }
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  @override
  void dispose() {
    _usernameController.dispose();
    _passwordController.dispose();
    _confirmPasswordController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(title: Text(_isLogin ? l.login : l.register)),
      body: Padding(
        padding: EdgeInsets.all(AppTheme.sp16),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            TextField(
              controller: _usernameController,
              decoration: InputDecoration(labelText: l.username),
            ),
            const SizedBox(height: 16),
            TextField(
              controller: _passwordController,
              decoration: InputDecoration(labelText: l.password),
              obscureText: true,
            ),
            if (!_isLogin) ...[
              const SizedBox(height: 16),
              TextField(
                controller: _confirmPasswordController,
                decoration: InputDecoration(labelText: l.confirmPassword),
                obscureText: true,
              ),
            ],
            const SizedBox(height: 32),
            _isLoading
                ? const CircularProgressIndicator()
                : ElevatedButton(
                    onPressed: _submit,
                    child: Text(_isLogin ? l.login : l.register),
                  ),
            TextButton(
              onPressed: () {
                setState(() {
                  _isLogin = !_isLogin;
                });
              },
              child: Text(_isLogin ? l.register : l.login),
            ),
          ],
        ),
      ),
    );
  }
}
