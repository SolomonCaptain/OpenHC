class HealthResponse {
  final String status;
  final bool cppLibraryAvailable;
  final String service;

  HealthResponse({
    required this.status,
    required this.cppLibraryAvailable,
    required this.service
  });

  factory HealthResponse.fromJson(Map<String, dynamic> json) {
    return HealthResponse(
      status: json['status'] as String,
      cppLibraryAvailable: json['cppLibraryAvailable'] as bool,
      service: json['service'] as String,
    );
  }
}