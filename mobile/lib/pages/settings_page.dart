import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:crypto/crypto.dart';
import 'package:http/http.dart' as http;
import 'package:image_picker/image_picker.dart';
import 'package:uuid/uuid.dart';
import '../l10n/app_localizations.dart';
import '../services/locale_service.dart';
import '../services/user_service.dart';
import '../services/base_service.dart';
import '../theme/app_theme.dart';

class SettingsPage extends StatefulWidget {
  const SettingsPage({super.key});

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  final _userService = UserService();

  Map<String, dynamic>? _profile;
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _loadProfile();
  }

  Future<void> _loadProfile() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final profile = await _userService.getUserProfile();
      if (mounted) {
        setState(() {
          _profile = profile;
          _loading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = e.toString();
          _loading = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(title: Text(l.settings)),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
          ? Center(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Text(_error!, textAlign: TextAlign.center),
                  const SizedBox(height: 16),
                  ElevatedButton(onPressed: _loadProfile, child: Text(l.retry)),
                ],
              ),
            )
          : SingleChildScrollView(
              padding: const EdgeInsets.all(AppTheme.sp16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _buildAvatarHeader(),
                  const SizedBox(height: AppTheme.sp24),

                  // Nickname
                  _SettingsCard(
                    icon: Icons.person_outline,
                    title: l.nickname,
                    trailing: ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 180),
                      child: Text(
                        _profile?['username'] ?? '',
                        style: const TextStyle(
                          color: AppTheme.textSecondary,
                          fontSize: 14,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    onTap: () => _showNicknameDialog(context),
                  ),
                  const SizedBox(height: 12),

                  // Email
                  _SettingsCard(
                    icon: Icons.email_outlined,
                    title: l.emailLabel,
                    trailing: ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 180),
                      child: Text(
                        _profile?['email'] ?? l.notSet,
                        style: const TextStyle(
                          color: AppTheme.textSecondary,
                          fontSize: 14,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    onTap: () => _showEmailDialog(context),
                  ),
                  const SizedBox(height: 12),

                  // Language
                  _SettingsCard(
                    icon: Icons.language,
                    title: l.language,
                    trailing: Text(
                      context.localeNotifier().locale.languageCode == 'zh'
                          ? l.chinese
                          : l.english,
                      style: const TextStyle(color: AppTheme.textSecondary),
                    ),
                    onTap: () => _showLanguageDialog(context),
                  ),
                  const SizedBox(height: 24),

                  // User agreement section
                  _SectionHeader(title: l.userAgreement),
                  const SizedBox(height: 8),

                  _SettingsCard(
                    icon: Icons.description_outlined,
                    title: l.userAgreement,
                    subtitle: l.userAgreementSubtitle,
                    onTap: () => _showAboutDialog(context),
                  ),

                  const SizedBox(height: 32),
                ],
              ),
            ),
    );
  }

  Widget _buildAvatarHeader() {
    final avatarUrl = _profile?['avatar_url'] as String?;
    final username = _profile?['username'] as String? ?? '';

    return Center(
      child: GestureDetector(
        onTap: () => _pickAndUploadAvatar(context),
        child: Stack(
          children: [
            CircleAvatar(
              radius: 52,
              backgroundColor: AppTheme.primary.withValues(alpha: 0.15),
              backgroundImage: avatarUrl != null && avatarUrl.isNotEmpty
                  ? NetworkImage(avatarUrl)
                  : null,
              child: avatarUrl == null || avatarUrl.isEmpty
                  ? Text(
                      username.isNotEmpty ? username[0].toUpperCase() : '?',
                      style: const TextStyle(
                        fontSize: 40,
                        fontWeight: FontWeight.bold,
                        color: AppTheme.primary,
                      ),
                    )
                  : null,
            ),
            Positioned(
              right: 0,
              bottom: 0,
              child: Container(
                padding: const EdgeInsets.all(6),
                decoration: const BoxDecoration(
                  color: AppTheme.primary,
                  shape: BoxShape.circle,
                ),
                child: const Icon(
                  Icons.camera_alt,
                  size: 18,
                  color: Colors.white,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _pickAndUploadAvatar(BuildContext context) async {
    final l = AppLocalizations.of(context)!;
    final messenger = ScaffoldMessenger.of(context);
    final source = await showModalBottomSheet<ImageSource>(
      context: context,
      builder: (ctx) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              leading: const Icon(Icons.photo_library),
              title: Text(l.gallery),
              onTap: () => Navigator.pop(ctx, ImageSource.gallery),
            ),
            ListTile(
              leading: const Icon(Icons.camera_alt),
              title: Text(l.camera),
              onTap: () => Navigator.pop(ctx, ImageSource.camera),
            ),
          ],
        ),
      ),
    );
    if (source == null) return;

    final picker = ImagePicker();
    final pickedFile = await picker.pickImage(
      source: source,
      maxWidth: 512,
      maxHeight: 512,
      imageQuality: 80,
    );
    if (pickedFile == null) return;

    try {
      messenger.showSnackBar(SnackBar(content: Text('${l.uploading}...')));

      // 1. Get STS token
      final stsToken = await _userService.getUploadToken();

      // 2. Read image bytes (works on mobile and web)
      final imageBytes = await pickedFile.readAsBytes();

      // 3. Generate object key
      final userId = _profile?['user_id'] ?? 'unknown';
      final ext = pickedFile.path.split('.').last.toLowerCase();
      final objectKey = 'avatars/$userId/${const Uuid().v4()}.$ext';

      // 4. Build OSS URL (endpoint may already include protocol)
      final endpointHost = stsToken.endpoint
          .replaceFirst(RegExp(r'^https?://'), '')
          .replaceAll(RegExp(r'/$'), '');
      final ossUrl = Uri.https('${stsToken.bucket}.$endpointHost', objectKey);

      final contentType = 'image/${ext == 'jpg' ? 'jpeg' : ext}';
      final ossDate = _buildOssDateHeader();
      final authorization = _buildOssAuthorization(
        method: 'PUT',
        contentType: contentType,
        date: ossDate,
        bucket: stsToken.bucket,
        objectKey: objectKey,
        accessKeyId: stsToken.accessKeyId,
        accessKeySecret: stsToken.accessKeySecret,
        securityToken: stsToken.securityToken,
      );

      // 5. PUT to OSS with STS token header
      final ossResponse = await http
          .put(
            ossUrl,
            headers: {
              'Date': ossDate,
              'Authorization': authorization,
              'x-oss-security-token': stsToken.securityToken,
              'Content-Type': contentType,
            },
            body: imageBytes,
          )
          .timeout(const Duration(seconds: 30));

      if (ossResponse.statusCode != 200) {
        throw Exception('OSS upload failed: ${ossResponse.statusCode}');
      }

      // 6. Update profile with avatar URL
      final avatarUrl = ossUrl.toString();
      final updated = await _userService.updateProfile(avatarUrl: avatarUrl);
      if (mounted) {
        setState(() => _profile = updated);
        messenger.showSnackBar(SnackBar(content: Text(l.avatarUpdated)));
      }
    } catch (e) {
      if (mounted) {
        messenger.showSnackBar(
          SnackBar(content: Text('${l.uploadFailed}: $e')),
        );
      }
    }
  }

  String _buildOssDateHeader() {
    const weekdays = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
    const months = [
      'Jan',
      'Feb',
      'Mar',
      'Apr',
      'May',
      'Jun',
      'Jul',
      'Aug',
      'Sep',
      'Oct',
      'Nov',
      'Dec',
    ];

    final now = DateTime.now().toUtc();
    final weekday = weekdays[now.weekday - 1];
    final day = now.day.toString().padLeft(2, '0');
    final month = months[now.month - 1];
    final year = now.year;
    final hour = now.hour.toString().padLeft(2, '0');
    final minute = now.minute.toString().padLeft(2, '0');
    final second = now.second.toString().padLeft(2, '0');
    return '$weekday, $day $month $year $hour:$minute:$second GMT';
  }

  String _buildOssAuthorization({
    required String method,
    required String contentType,
    required String date,
    required String bucket,
    required String objectKey,
    required String accessKeyId,
    required String accessKeySecret,
    required String securityToken,
  }) {
    final canonicalHeaders = 'x-oss-security-token:$securityToken\n';
    final canonicalResource = '/$bucket/$objectKey';
    final stringToSign =
        '$method\n\n$contentType\n$date\n$canonicalHeaders$canonicalResource';
    final hmac = Hmac(sha1, utf8.encode(accessKeySecret));
    final signature = base64Encode(
      hmac.convert(utf8.encode(stringToSign)).bytes,
    );
    return 'OSS $accessKeyId:$signature';
  }

  void _showNicknameDialog(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    final controller = TextEditingController(text: _profile?['username'] ?? '');
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l.nicknameChange),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: controller,
              decoration: InputDecoration(
                labelText: l.nickname,
                hintText: l.nicknameChangeHint,
              ),
              maxLength: 50,
              autofocus: true,
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: Text(l.cancel),
          ),
          ElevatedButton(
            onPressed: () async {
              final nickname = controller.text.trim();
              if (nickname.isEmpty) {
                ScaffoldMessenger.of(
                  context,
                ).showSnackBar(SnackBar(content: Text(l.nicknameEmpty)));
                return;
              }
              Navigator.pop(ctx);
              await _updateNickname(context, nickname);
            },
            child: Text(l.confirm),
          ),
        ],
      ),
    );
  }

  Future<void> _updateNickname(BuildContext context, String nickname) async {
    final l = AppLocalizations.of(context)!;
    final messenger = ScaffoldMessenger.of(context);
    try {
      final updated = await _userService.updateProfile(username: nickname);
      if (mounted) {
        setState(() => _profile = updated);
        messenger.showSnackBar(
          SnackBar(content: Text(l.nicknameChangeSuccess)),
        );
      }
    } on ConflictException catch (e) {
      if (mounted) {
        messenger.showSnackBar(SnackBar(content: Text(e.message)));
      }
    } catch (e) {
      if (mounted) {
        messenger.showSnackBar(
          SnackBar(content: Text(l.operationFailed(e.toString()))),
        );
      }
    }
  }

  void _showEmailDialog(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    final controller = TextEditingController(text: _profile?['email'] ?? '');
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l.emailChange),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: controller,
              decoration: InputDecoration(
                labelText: l.emailLabel,
                hintText: l.emailChangeHint,
              ),
              maxLength: 100,
              autofocus: true,
              keyboardType: TextInputType.emailAddress,
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: Text(l.cancel),
          ),
          ElevatedButton(
            onPressed: () async {
              final email = controller.text.trim();
              Navigator.pop(ctx);
              await _updateEmail(context, email);
            },
            child: Text(l.confirm),
          ),
        ],
      ),
    );
  }

  Future<void> _updateEmail(BuildContext context, String email) async {
    final l = AppLocalizations.of(context)!;
    final messenger = ScaffoldMessenger.of(context);
    if (email.isNotEmpty && !email.endsWith('@email.ncu.edu.cn')) {
      messenger.showSnackBar(SnackBar(content: Text(l.emailDomainError)));
      return;
    }
    try {
      final updated = await _userService.updateProfile(
        email: email.isEmpty ? null : email,
      );
      if (mounted) {
        setState(() => _profile = updated);
        messenger.showSnackBar(SnackBar(content: Text(l.emailChangeSuccess)));
      }
    } on ConflictException catch (e) {
      if (mounted) {
        messenger.showSnackBar(SnackBar(content: Text(e.message)));
      }
    } catch (e) {
      if (mounted) {
        messenger.showSnackBar(
          SnackBar(content: Text(l.operationFailed(e.toString()))),
        );
      }
    }
  }

  void _showLanguageDialog(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l.language),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              title: Text(l.english),
              onTap: () {
                ctx.localeNotifier().setLocale(const Locale('en'));
                Navigator.pop(ctx);
              },
            ),
            ListTile(
              title: Text(l.chinese),
              onTap: () {
                ctx.localeNotifier().setLocale(const Locale('zh'));
                Navigator.pop(ctx);
              },
            ),
          ],
        ),
      ),
    );
  }

  void _showAboutDialog(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    showAboutDialog(
      context: context,
      applicationName: l.appTitle,
      applicationVersion: '1.0.0',
      applicationLegalese: l.platformDisclaimer,
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;
  const _SectionHeader({required this.title});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(left: 4, bottom: 4),
      child: Text(
        title,
        style: const TextStyle(
          fontSize: 13,
          fontWeight: FontWeight.w600,
          color: AppTheme.textSecondary,
        ),
      ),
    );
  }
}

class _SettingsCard extends StatelessWidget {
  final IconData icon;
  final String title;
  final String? subtitle;
  final Widget? trailing;
  final VoidCallback onTap;

  const _SettingsCard({
    required this.icon,
    required this.title,
    this.subtitle,
    this.trailing,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: EdgeInsets.zero,
      child: ListTile(
        contentPadding: const EdgeInsets.symmetric(
          horizontal: AppTheme.sp16,
          vertical: AppTheme.sp8,
        ),
        leading: Container(
          padding: const EdgeInsets.all(10),
          decoration: BoxDecoration(
            color: AppTheme.primary.withValues(alpha: 0.1),
            borderRadius: BorderRadius.circular(10),
          ),
          child: Icon(icon, color: AppTheme.primary),
        ),
        title: Text(title, style: const TextStyle(fontWeight: FontWeight.w600)),
        subtitle: subtitle != null
            ? Text(
                subtitle!,
                style: const TextStyle(
                  fontSize: 12,
                  color: AppTheme.textSecondary,
                ),
              )
            : null,
        trailing:
            trailing ??
            const Icon(Icons.chevron_right, color: AppTheme.textSecondary),
        onTap: onTap,
      ),
    );
  }
}
