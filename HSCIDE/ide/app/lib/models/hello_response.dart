class HelloResponse {
  final String message;
  final String source;

  HelloResponse({required this.message, required this.source});

  factory HelloResponse.fromJson(Map<String, dynamic> json) {
    return HelloResponse(
      message: json['message'] as String,
      source: json['source'] as String,
    );
  }
}