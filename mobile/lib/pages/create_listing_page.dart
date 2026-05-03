import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:image_picker/image_picker.dart';
import 'package:provider/provider.dart';
import '../l10n/app_localizations.dart';
import 'package:go_router/go_router.dart';
import '../services/api_service.dart';
import '../theme/app_theme.dart';

class CreateListingPage extends StatefulWidget {
  final ApiService? apiService;

  const CreateListingPage({super.key, this.apiService});

  @override
  State<CreateListingPage> createState() => _CreateListingPageState();
}

class _CreateListingPageState extends State<CreateListingPage> {
  final _formKey = GlobalKey<FormState>();
  late final ApiService _apiService;
  final _imagePicker = ImagePicker();

  final _titleController = TextEditingController();
  final _brandController = TextEditingController();
  final _priceController = TextEditingController();
  final _descriptionController = TextEditingController();

  String _category = 'electronics';
  int _conditionScore = 7;
  final List<String> _defects = [];
  final _defectController = TextEditingController();
  bool _isLoading = false;
  bool _isRecognizing = false;
  String? _imageBase64;

  static const _categoryKeys = [
    'electronics',
    'books',
    'digitalAccessories',
    'dailyGoods',
    'clothingShoes',
    'other',
  ];

  String _getCategoryDisplayName(BuildContext context, String key) {
    final l = AppLocalizations.of(context)!;
    switch (key) {
      case 'electronics':
        return l.electronics;
      case 'books':
        return l.books;
      case 'digitalAccessories':
        return l.digitalAccessories;
      case 'dailyGoods':
        return l.dailyGoods;
      case 'clothingShoes':
        return l.clothingShoes;
      case 'other':
        return l.other;
      default:
        return key;
    }
  }

  @override
  void initState() {
    super.initState();
    _apiService = widget.apiService ?? context.read<ApiService>();
  }

  @override
  void dispose() {
    _titleController.dispose();
    _brandController.dispose();
    _priceController.dispose();
    _descriptionController.dispose();
    _defectController.dispose();
    super.dispose();
  }

  Future<void> _takePhotoAndRecognize() async {
    try {
      final XFile? image = await _imagePicker.pickImage(
        source: ImageSource.camera,
        imageQuality: 80,
        maxWidth: 1024,
      );

      if (image == null) return;

      final bytes = await image.readAsBytes();
      final base64 = base64Encode(bytes);

      setState(() {
        _isRecognizing = true;
        _imageBase64 = base64;
      });

      final result = await _apiService.recognizeItem(base64);

      setState(() {
        _titleController.text = result.title;
        _brandController.text = result.brand;
        _category = result.category;
        _conditionScore = result.conditionScore.clamp(1, 10);
        _defects.clear();
        _defects.addAll(result.defects);
        _descriptionController.text = result.description;
        _isRecognizing = false;
      });

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('识别成功，已自动填充信息')),
        );
      }
    } catch (e) {
      setState(() => _isRecognizing = false);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('识别失败: $e')),
        );
      }
    }
  }

  Future<void> _pickAndRecognize() async {
    try {
      final XFile? image = await _imagePicker.pickImage(
        source: ImageSource.gallery,
        imageQuality: 80,
        maxWidth: 1024,
      );

      if (image == null) return;

      final bytes = await image.readAsBytes();
      final base64 = base64Encode(bytes);

      setState(() {
        _isRecognizing = true;
        _imageBase64 = base64;
      });

      final result = await _apiService.recognizeItem(base64);

      setState(() {
        _titleController.text = result.title;
        _brandController.text = result.brand;
        _category = result.category;
        _conditionScore = result.conditionScore.clamp(1, 10);
        _defects.clear();
        _defects.addAll(result.defects);
        _descriptionController.text = result.description;
        _isRecognizing = false;
      });

      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('识别成功，已自动填充信息')),
        );
      }
    } catch (e) {
      setState(() => _isRecognizing = false);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('识别失败: $e')),
        );
      }
    }
  }

  Future<void> _submit() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() => _isLoading = true);

    try {
      final price = double.tryParse(_priceController.text) ?? 0;
      final id = await _apiService.createListing(
        title: _titleController.text.trim(),
        category: _category,
        brand: _brandController.text.trim(),
        conditionScore: _conditionScore,
        suggestedPriceCny: price,
        defects: _defects,
        description: _descriptionController.text.trim().isEmpty
            ? null
            : _descriptionController.text.trim(),
      );

      if (mounted) {
        final l = AppLocalizations.of(context)!;
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(l.createSuccess)),
        );
        context.go('/listing/$id');
      }
    } catch (e) {
      if (mounted) {
        final l = AppLocalizations.of(context)!;
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('${l.createError}: $e')),
        );
        setState(() => _isLoading = false);
      }
    }
  }

  void _addDefect() {
    final text = _defectController.text.trim();
    if (text.isNotEmpty && !_defects.contains(text)) {
      setState(() => _defects.add(text));
      _defectController.clear();
    }
  }

  String _getConditionLabel() {
    if (_conditionScore >= 9) return 'Like New';
    if (_conditionScore >= 7) return 'Good';
    if (_conditionScore >= 5) return 'Fair';
    return 'Poor';
  }

  Color get _conditionColor => AppTheme.conditionColor(_conditionScore);

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context)!;
    return Scaffold(
      appBar: AppBar(
        title: Text(l.createListing),
        leading: IconButton(
          icon: const Icon(Icons.close),
          onPressed: () => context.pop(),
        ),
        actions: [
          if (_isRecognizing)
            const Padding(
              padding: EdgeInsets.all(16),
              child: SizedBox(
                width: 20,
                height: 20,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
            )
          else
            PopupMenuButton<String>(
              icon: const Icon(Icons.camera_alt),
              tooltip: 'AI识别',
              onSelected: (value) {
                if (value == 'camera') {
                  _takePhotoAndRecognize();
                } else if (value == 'gallery') {
                  _pickAndRecognize();
                }
              },
              itemBuilder: (context) => [
                PopupMenuItem(
                  value: 'camera',
                  child: Row(
                    children: [
                      const Icon(Icons.camera_alt, size: 20),
                      const SizedBox(width: 8),
                      Text('拍照识别'),
                    ],
                  ),
                ),
                PopupMenuItem(
                  value: 'gallery',
                  child: Row(
                    children: [
                      const Icon(Icons.photo_library, size: 20),
                      const SizedBox(width: 8),
                      Text('相册识别'),
                    ],
                  ),
                ),
              ],
            ),
        ],
      ),
      body: Form(
        key: _formKey,
        child: ListView(
          padding: const EdgeInsets.all(AppTheme.sp16),
          children: [
            // AI recognition hint
            if (_imageBase64 == null)
              Container(
                padding: const EdgeInsets.all(AppTheme.sp16),
                decoration: BoxDecoration(
                  color: AppTheme.primary.withValues(alpha: 0.08),
                  borderRadius: BorderRadius.circular(AppTheme.radiusMd),
                  border: Border.all(
                    color: AppTheme.primary.withValues(alpha: 0.2),
                    style: BorderStyle.solid,
                  ),
                ),
                child: Column(
                  children: [
                    Icon(
                      Icons.auto_awesome,
                      size: 40,
                      color: AppTheme.primary.withValues(alpha: 0.6),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      '点击右上角相机图标拍照或选择图片',
                      textAlign: TextAlign.center,
                      style: TextStyle(
                        color: AppTheme.primary.withValues(alpha: 0.8),
                        fontSize: 14,
                      ),
                    ),
                    Text(
                      'AI将自动识别商品信息',
                      textAlign: TextAlign.center,
                      style: TextStyle(
                        color: AppTheme.primary.withValues(alpha: 0.6),
                        fontSize: 12,
                      ),
                    ),
                    const SizedBox(height: 12),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        TextButton.icon(
                          onPressed: _takePhotoAndRecognize,
                          icon: const Icon(Icons.camera_alt, size: 18),
                          label: const Text('拍照'),
                        ),
                        const SizedBox(width: 16),
                        TextButton.icon(
                          onPressed: _pickAndRecognize,
                          icon: const Icon(Icons.photo_library, size: 18),
                          label: const Text('相册'),
                        ),
                      ],
                    ),
                  ],
                ),
              )
            else
              Container(
                height: 120,
                width: double.infinity,
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(AppTheme.radiusMd),
                  color: AppTheme.success.withValues(alpha: 0.1),
                ),
                child: Stack(
                  children: [
                    ClipRRect(
                      borderRadius: BorderRadius.circular(AppTheme.radiusMd),
                      child: Image.memory(
                        base64Decode(_imageBase64!),
                        height: 120,
                        width: double.infinity,
                        fit: BoxFit.cover,
                      ),
                    ),
                    Positioned(
                      top: 8,
                      right: 8,
                      child: IconButton.filled(
                        onPressed: () => setState(() => _imageBase64 = null),
                        icon: const Icon(Icons.close, size: 18),
                        style: IconButton.styleFrom(
                          backgroundColor: Colors.black54,
                          foregroundColor: Colors.white,
                          padding: const EdgeInsets.all(4),
                          minimumSize: const Size(28, 28),
                        ),
                      ),
                    ),
                    Positioned(
                      bottom: 8,
                      left: 8,
                      child: Container(
                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                        decoration: BoxDecoration(
                          color: AppTheme.success,
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: const Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(Icons.check_circle, size: 14, color: Colors.white),
                            SizedBox(width: 4),
                            Text(
                              '已识别',
                              style: TextStyle(color: Colors.white, fontSize: 12),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            const SizedBox(height: AppTheme.sp16),
            TextFormField(
              controller: _titleController,
              decoration: InputDecoration(
                labelText: '${l.title} *',
                hintText: 'e.g. iPhone 13 Pro Max 256G',
              ),
              validator: (v) => v == null || v.trim().isEmpty ? l.titleRequired : null,
            ),
            const SizedBox(height: AppTheme.sp16),
            DropdownButtonFormField<String>(
              // ignore: deprecated_member_use
              value: _category,
              decoration: InputDecoration(labelText: '${l.category} *'),
              items: _categoryKeys
                  .map((c) => DropdownMenuItem(value: c, child: Text(_getCategoryDisplayName(context, c))))
                  .toList(),
              onChanged: (v) => setState(() => _category = v!),
            ),
            const SizedBox(height: AppTheme.sp16),
            TextFormField(
              controller: _brandController,
              decoration: InputDecoration(
                labelText: '${l.brand} *',
                hintText: 'e.g. Apple',
              ),
              validator: (v) => v == null || v.trim().isEmpty ? 'Please enter brand' : null,
            ),
            const SizedBox(height: AppTheme.sp16),
            TextFormField(
              controller: _priceController,
              decoration: InputDecoration(
                labelText: '${l.price} (CNY) *',
                hintText: '0.00',
                prefixText: '¥ ',
              ),
              keyboardType: const TextInputType.numberWithOptions(decimal: true),
              validator: (v) {
                if (v == null || v.isEmpty) return 'Please enter price';
                if (double.tryParse(v) == null) return 'Please enter a valid price';
                return null;
              },
            ),
            const SizedBox(height: AppTheme.sp20),
            Text(
              l.condition,
              style: const TextStyle(fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(
                  child: Slider(
                    value: _conditionScore.toDouble(),
                    min: 1,
                    max: 10,
                    divisions: 9,
                    label: '$_conditionScore/10',
                    onChanged: (v) => setState(() => _conditionScore = v.round()),
                  ),
                ),
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                  decoration: BoxDecoration(
                    color: _conditionColor.withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(
                    '$_conditionScore/10 ${_getConditionLabel()}',
                    style: TextStyle(
                      fontWeight: FontWeight.w600,
                      color: _conditionColor,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: AppTheme.sp20),
            Text(
              l.defects,
              style: const TextStyle(fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _defectController,
                    decoration: const InputDecoration(
                      hintText: 'e.g. Minor screen scratch',
                    ),
                    onSubmitted: (_) => _addDefect(),
                  ),
                ),
                const SizedBox(width: 8),
                IconButton.filled(
                  onPressed: _addDefect,
                  icon: const Icon(Icons.add),
                ),
              ],
            ),
            if (_defects.isNotEmpty) ...[
              const SizedBox(height: 12),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: _defects
                    .map((d) => Chip(
                          label: Text(d),
                          deleteIcon: const Icon(Icons.close, size: 16),
                          onDeleted: () => setState(() => _defects.remove(d)),
                          backgroundColor: AppTheme.error.withValues(alpha: 0.1),
                          labelStyle: const TextStyle(color: AppTheme.error),
                          side: BorderSide.none,
                        ))
                    .toList(),
              ),
            ],
            const SizedBox(height: AppTheme.sp16),
            TextFormField(
              controller: _descriptionController,
              decoration: InputDecoration(
                labelText: '${l.description} (optional)',
                hintText: 'Describe the item condition, usage, etc.',
                alignLabelWithHint: true,
              ),
              maxLines: 4,
            ),
            const SizedBox(height: AppTheme.sp32),
            SizedBox(
              height: 52,
              child: ElevatedButton(
                onPressed: _isLoading ? null : _submit,
                child: _isLoading
                    ? const SizedBox(
                        height: 20,
                        width: 20,
                        child: CircularProgressIndicator(
                          strokeWidth: 2,
                          color: Colors.white,
                        ),
                      )
                    : Text(l.submit),
              ),
            ),
            const SizedBox(height: AppTheme.sp32),
          ],
        ),
      ),
    );
  }
}
