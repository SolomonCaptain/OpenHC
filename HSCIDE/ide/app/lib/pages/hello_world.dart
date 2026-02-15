import 'package:flutter/material.dart';
import '../models/hello_response.dart';
import '../services/api_service.dart';

class HelloWorld extends StatefulWidget {
  const HelloWorld({super.key});

  @override
  State<HelloWorld> createState() => _HelloWorldState();
}

class _HelloWorldState extends State<HelloWorld> {
  final ApiService _apiService = ApiService();
  HelloResponse? _helloData;
  bool _loading = false;
  String? _error;
  Map<String, bool>? _serviceStatus;

  @override
  void initState() {
    super.initState();
    _checkServiceStatus();
  }

  Future<void> _checkServiceStatus() async {
    final status = await _apiService.checkServiceStatus();
    setState(() {
      _serviceStatus = status;
    });
  }

  Future<void> _fetchHelloWorld() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final data = await _apiService.getHello();
      setState(() {
        _helloData = data;
      });
    } catch (e) {
      setState(() {
        _error = e.toString();
      });
    } finally {
      setState(() {
        _loading = false;
      });
    }
  }

  Color _getSourceColor(String source) {
    return source.contains('C++') ? Colors.green : Colors.blue;
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Container(
        color: Colors.grey[100],
        child: Center(
          child: Padding(
            padding: const EdgeInsets.all(24.0),
            child: Card(
              elevation: 4,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(12),
              ),
              child: Padding(
                padding: const EdgeInsets.all(32.0),
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 600),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        'FastAPI + C++ + React',
                        style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                          fontWeight: FontWeight.bold,
                        ),
                        textAlign: TextAlign.center,
                      ),
                      const SizedBox(height: 8),
                      Text(
                        'Full-stack Hello World Application',
                        style: Theme.of(context).textTheme.titleMedium?.copyWith(
                          color: Colors.grey[600],
                        ),
                        textAlign: TextAlign.center,
                      ),
                      const SizedBox(height: 24),

                      // 服务状态指示
                      if (_serviceStatus != null)
                        Container(
                          margin: const EdgeInsets.only(bottom: 24),
                          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                          decoration: BoxDecoration(
                            color: _serviceStatus!['isRunning']! ? Colors.green[50] : Colors.red[50],
                            borderRadius: BorderRadius.circular(8),
                            border: Border.all(
                              color: _serviceStatus!['isRunning']! ? Colors.green : Colors.red,
                            ),
                          ),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              Icon(
                                _serviceStatus!['isRunning']! ? Icons.check_circle : Icons.error,
                                color: _serviceStatus!['isRunning']! ? Colors.green : Colors.red,
                              ),
                              const SizedBox(width: 8),
                              Text(
                                'Backend Status: ${_serviceStatus!['isRunning']! ? 'Running' : 'Not Running'}',
                                style: TextStyle(
                                  color: _serviceStatus!['isRunning']! ? Colors.green[900] : Colors.red[900],
                                  fontWeight: FontWeight.w500,
                                ),
                              ),
                              if (_serviceStatus!['isRunning']!)
                                Container(
                                  margin: const EdgeInsets.only(left: 16),
                                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                                  decoration: BoxDecoration(
                                    color: _serviceStatus!['cppAvailable']! ? Colors.green : Colors.orange,
                                    borderRadius: BorderRadius.circular(16),
                                  ),
                                  child: Text(
                                    'C++ Library: ${_serviceStatus!['cppAvailable']! ? 'Available' : 'Not Available'}',
                                    style: const TextStyle(
                                      color: Colors.white,
                                      fontSize: 12,
                                    ),
                                  ),
                                ),
                            ],
                          ),
                        ),

                      // 按钮
                      ElevatedButton(
                        onPressed: _loading ? null : _fetchHelloWorld,
                        style: ElevatedButton.styleFrom(
                          padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 16),
                          textStyle: const TextStyle(fontSize: 18),
                        ),
                        child: _loading
                            ? const SizedBox(
                          height: 20,
                          width: 20,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                            : const Text('Get Message'),
                      ),
                      const SizedBox(height: 24),

                      // 错误提示
                      if (_error != null)
                        Container(
                          width: double.infinity,
                          padding: const EdgeInsets.all(12),
                          decoration: BoxDecoration(
                            color: Colors.red[50],
                            borderRadius: BorderRadius.circular(8),
                            border: Border.all(color: Colors.red),
                          ),
                          child: Row(
                            children: [
                              Icon(Icons.error, color: Colors.red[700]),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Text(
                                  _error!,
                                  style: TextStyle(color: Colors.red[900]),
                                ),
                              ),
                            ],
                          ),
                        ),

                      // 消息卡片
                      if (_helloData != null)
                        Container(
                          width: double.infinity,
                          margin: const EdgeInsets.only(top: 16),
                          padding: const EdgeInsets.all(20),
                          decoration: BoxDecoration(
                            border: Border.all(color: Colors.grey.shade300),
                            borderRadius: BorderRadius.circular(12),
                          ),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Row(
                                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                                children: [
                                  Text(
                                    'Message Received',
                                    style: Theme.of(context).textTheme.titleLarge,
                                  ),
                                  Container(
                                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                                    decoration: BoxDecoration(
                                      color: _getSourceColor(_helloData!.source).withOpacity(0.2),
                                      borderRadius: BorderRadius.circular(16),
                                    ),
                                    child: Text(
                                      _helloData!.source,
                                      style: TextStyle(
                                        color: _getSourceColor(_helloData!.source),
                                        fontWeight: FontWeight.bold,
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                              const SizedBox(height: 16),
                              Center(
                                child: Text(
                                  _helloData!.message,
                                  style: const TextStyle(
                                    fontSize: 32,
                                    fontWeight: FontWeight.bold,
                                    color: Colors.blue,
                                  ),
                                ),
                              ),
                              const SizedBox(height: 16),
                              Text(
                                'This message was generated by ${_helloData!.source.contains('C++') ? 'a C++ dynamic library' : 'Python fallback'} and served through FastAPI to your Flutter frontend.',
                                style: const TextStyle(color: Colors.grey),
                              ),
                            ],
                          ),
                        ),

                      const SizedBox(height: 24),
                      Text(
                        'Stack: Flutter + Material + FastAPI + C++',
                        style: TextStyle(color: Colors.grey[600]),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}