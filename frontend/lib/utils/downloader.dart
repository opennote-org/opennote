import 'downloader_stub.dart'
    if (dart.library.html) 'downloader_web.dart'
    if (dart.library.io) 'downloader_io.dart';

Future<void> downloadFile(List<int> bytes, String fileName) async {
  return downloadZip(bytes, fileName);
}
