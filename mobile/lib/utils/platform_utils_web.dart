// Implementation for web platform.

String getApiBaseUrl() {
  // Web always uses localhost (user's development server)
  return 'http://localhost:3000';
}

String getWsUrl() {
  // Web uses ws://localhost
  return 'ws://localhost:3000/api/ws';
}
