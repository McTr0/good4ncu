// Stub implementation for unsupported platforms.
// This file is used as fallback when neither dart:io nor dart:html is available.
String getApiBaseUrl() => 'http://localhost:3000';
String getWsUrl() => 'ws://localhost:3000/api/ws';
