import 'dart:convert';
import 'package:http/http.dart' as http;
import '../models/hello_response.dart';
import '../models/health_response.dart';

const String baseUrl = 'http://154.37.219.104:8000';

class ApiService {
  static final ApiService _instance = ApiService._internal();
  factory ApiService() => _instance;
  ApiService._internal();

  Future<HelloResponse> getHello() async {
    final response = await http.get(Uri.parse('$baseUrl/api/hello'));
    if (response.statusCode == 200) {
      return HelloResponse.fromJson(jsonDecode(response.body));
    } else {
      throw Exception('Failed to load hello message');
    }
  }
  
  Future<HealthResponse> checkHealth() async {
    final response = await http.get(Uri.parse('$baseUrl/api/health'));
    if (response.statusCode == 200) {
      return HealthResponse.fromJson(jsonDecode(response.body));
    } else {
      throw Exception('Failed to check health');
    }
  }

  Future<Map<String, bool>> checkServiceStatus() async {
    try {
      final health = await checkHealth();
      return {
        'isRunning': health.status == 'healthy',
        'cppLibraryAvailable': health.cppLibraryAvailable,
      };
    } catch (e) {
      return {
        'isRunning': false,
        'cppLibraryAvailable': false,
      };
    }
  }
}