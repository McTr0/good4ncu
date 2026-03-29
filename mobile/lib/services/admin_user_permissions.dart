bool canAdminImpersonateUser(Map<String, dynamic> user) {
  final role = user['role']?.toString().toLowerCase();
  final status = user['status']?.toString().toLowerCase();
  final isBannedFlag = user['is_banned'] == true;

  if (role == null || role.isEmpty) return false;
  if (status == null || status.isEmpty) return false;

  return role != 'admin' && status == 'active' && !isBannedFlag;
}
