import 'package:flutter/foundation.dart';
import 'package:notes/state/services.dart';

mixin Users on ChangeNotifier, Services {
  String? username;
}