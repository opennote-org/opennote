import 'dart:io';
import 'package:file_picker/file_picker.dart';

Future<void> downloadZip(List<int> bytes, String fileName) async {
  String? outputFile = await FilePicker.platform.saveFile(
    dialogTitle: 'Please select an output file:',
    fileName: fileName,
  );

  if (outputFile != null) {
    final file = File(outputFile);
    await file.writeAsBytes(bytes);
  }
}
